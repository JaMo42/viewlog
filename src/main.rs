use chrono::{DateTime, Local};
use clap::Parser;
use notify::{recommended_watcher, Event, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::{
    fs::File,
    io::{self, stdout, Read, Seek, SeekFrom, Write},
    path::Path,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod noecho;
use noecho::NoEcho;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn goto(line: usize, col: usize) {
    print!("\x1b[{};{}H", 1 + line, 1 + col);
}

fn clear_screen(discard_old: bool) {
    // Clear screen
    print!("\x1b[2J");
    if discard_old {
        // Clear scrollback buffer
        print!("\x1b[3J");
    }
    // Move to top-left corner
    goto(0, 0);
}

fn show_cursor(yay_or_nay: bool) {
    if yay_or_nay {
        print!("\x1b[?25h");
    } else {
        print!("\x1b[?25l");
    }
}

fn alternative_screen_buffer(enable: bool) {
    if enable {
        print!("\x1b[?1049h");
    } else {
        print!("\x1b[?1049l");
    }
}

fn repeat_ascii(char: char, times: usize) -> String {
    if !char.is_ascii() {
        panic!("repeat_ascii with non-ascii character");
    }
    String::from_utf8(vec![char as u8; times]).unwrap()
}

struct HideCursor;

impl HideCursor {
    fn begin() -> Self {
        show_cursor(false);
        Self
    }
}

impl Drop for HideCursor {
    fn drop(&mut self) {
        show_cursor(true);
    }
}

#[derive(Parser)]
struct Commandline {
    /// File to view
    file: String,

    /// Show timestamps when a line is printed
    #[arg(short, long, default_value_t = false)]
    timestamps: bool,

    /// Clear the scrollback buffer of the terminal when the file is truncated
    #[arg(short, long, default_value_t = false)]
    discard_old: bool,
}

struct CursorInfo {
    term_lines: usize,
    term_cols: usize,
    cursor_line: usize,
    cursor_col: usize,
    save_line: usize,
    save_col: usize,
}

impl CursorInfo {
    fn new() -> Self {
        let (term_cols, mut term_lines) = term_size::dimensions().unwrap();
        // Reserve one line for the status bar
        term_lines -= 1;
        Self {
            term_lines,
            term_cols,
            cursor_line: 0,
            cursor_col: 0,
            save_line: 0,
            save_col: 0,
        }
    }

    fn newline(&mut self) {
        if self.cursor_line != self.term_lines {
            self.cursor_line += 1;
        }
        self.cursor_col = 0;
    }

    fn add(&mut self, n: usize) {
        self.cursor_col += n;
    }

    fn save(&mut self) {
        self.save_line = self.cursor_line;
        self.save_col = self.cursor_col;
    }

    fn restore(&mut self) {
        self.cursor_line = self.save_line;
        self.cursor_col = self.save_col;
        goto(self.cursor_line, self.cursor_col);
    }

    fn fits(&self, cells: usize) -> bool {
        self.cursor_col + cells < self.term_cols
    }

    fn clear(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
    }
}

struct Viewer {
    file_name: String,
    timestamps: bool,
    discard_old: bool,
    line: Vec<char>,
    file: File,
    cursor: CursorInfo,
    time: DateTime<Local>,
    what_time: &'static str,
}

impl Viewer {
    fn new(args: &Commandline) -> io::Result<Self> {
        Ok(Self {
            file_name: if let Ok(home_dir) = std::env::var("HOME") {
                args.file.replace(&home_dir, "~")
            } else {
                args.file.clone()
            },
            timestamps: args.timestamps,
            discard_old: args.discard_old,
            line: Vec::new(),
            file: File::open(&args.file)?,
            cursor: CursorInfo::new(),
            time: Local::now(),
            what_time: "Started",
        })
    }

    fn on_change(&mut self) {
        let mut data = Vec::new();
        let old_position = self.file.stream_position().unwrap();
        if self.file.read_to_end(&mut data).is_ok() {
            self.file.seek(SeekFrom::End(0)).unwrap();
            let new_position = self.file.stream_position().unwrap();
            if new_position == 0 && old_position != 0 {
                self.truncate();
            } else {
                self.add_bytes(&data);
            }
        } else {
            panic!("Read failed");
        }
    }

    fn truncate(&mut self) {
        self.time = Local::now();
        self.what_time = "Created";
        clear_screen(self.discard_old);
        self.line.clear();
        self.cursor.clear();
        self.print_header(true);
        stdout().flush().ok();
    }

    fn add_bytes(&mut self, data: &[u8]) {
        for c in String::from_utf8_lossy(data).chars() {
            if c == '\n' {
                self.print_line();
            } else if c == '\r' {
                continue;
            } else {
                self.line.push(c);
            }
        }
    }

    fn print_escape(&self, mut i: usize) -> usize {
        let start = i;
        // '\x1b['
        i += 2;
        // Consume all following numbers and semicolons
        while i < self.line.len() {
            let c = self.line[i];
            if !(c.is_ascii_digit() || c == ';') {
                break;
            }
            i += 1;
        }
        // Terminating character
        i += 1;
        // Collect and print at once since most terminals don't let you print
        // escape sequences character by character.
        let seq: String = self.line[start..i].iter().collect();
        print!("{}", seq);
        i
    }

    fn newline(&mut self) {
        println!("\x1b[K");
        self.cursor.newline();
    }

    fn print_line(&mut self) {
        let timestamp_size;
        if self.timestamps {
            let now = Local::now();
            let timestamp = now.format("%H:%M:%S ").to_string();
            print!("\x1b[2m{}\x1b[0m", timestamp);
            timestamp_size = timestamp.width();
            self.cursor.add(timestamp_size);
        } else {
            timestamp_size = 0;
        }
        let timestamp_space = repeat_ascii(' ', timestamp_size);
        let mut i = 0;
        while i < self.line.len() {
            let c = self.line[i];
            if c == '\x1b' {
                i = self.print_escape(i);
                continue;
            }
            let w = c.width().unwrap_or(1);
            if !self.cursor.fits(w) {
                self.newline();
                print!("{}", timestamp_space);
                self.cursor.add(timestamp_size);
            }
            print!("{}", c);
            self.cursor.add(w);
            i += 1;
        }
        self.newline();
        self.line.clear();
        self.print_header(false);
        stdout().flush().ok();
    }

    fn print_header(&mut self, truncated: bool) {
        self.cursor.save();
        print!("\x1b[7m");

        goto(self.cursor.term_lines, 0);
        print!("{}", repeat_ascii(' ', self.cursor.term_cols));

        goto(self.cursor.term_lines, 1);
        print!("Viewing \x1b[1m{}\x1b[22m", self.file_name);

        if truncated {
            print!("   File truncated");
        }

        let time = format!("{} at {}", self.what_time, self.time.format("%H:%M:%S"));
        goto(
            self.cursor.term_lines,
            self.cursor.term_cols - time.len() - 1,
        );
        print!("{}", time);

        print!("\x1b[27m");
        self.cursor.restore();
    }
}

fn run() -> Result<()> {
    let cmdline = Commandline::parse();
    // Clear screen initially so we know the cursor position,
    // instead of bothering to read it using escape sequences.
    clear_screen(false);
    let mut viewer = Viewer::new(&cmdline)?;
    let _hide_cursor = HideCursor::begin();
    // This `print_line` causes a update even
    // if the viewed file is initially empty
    viewer.print_line();
    stdout().flush().ok();
    // Read initial content
    viewer.on_change();
    // Watch for changes
    let mut watcher =
        recommended_watcher(
            move |event_or_error: notify::Result<Event>| match event_or_error {
                Ok(event) => {
                    use notify::EventKind::Modify;
                    if matches!(event.kind, Modify(_)) {
                        viewer.on_change();
                    }
                }
                Err(error) => {
                    eprintln!("watch error: {error}");
                    alternative_screen_buffer(false);
                    std::process::exit(1);
                }
            },
        )?;
    let _no_echo = NoEcho::begin();
    watcher.watch(Path::new(&cmdline.file), RecursiveMode::NonRecursive)?;
    // Run until SIGINT, SIGTERM, or SIGHUP
    let (tx, rx) = channel();
    ctrlc::set_handler(move || {
        tx.send(()).ok();
    })?;
    rx.recv()?;
    Ok(())
}

fn main() {
    let mut code = 0;
    alternative_screen_buffer(true);
    if let Err(error) = run() {
        eprintln!("{error}");
        code = 1;
    }
    alternative_screen_buffer(false);
    std::process::exit(code);
}
