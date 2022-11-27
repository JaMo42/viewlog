# viewlog

Tiny log file viewer, similar to `tail -f`.

It works on Windows and Linux.

## Usage

```
$ viewlog [OPTIONS] <FILE>
```

## Options

- `-t`, `--timestamps` Show timestamps when a line is printed

- `-d`, `--discard-old` Clear the scrollback buffer of the terminal when the file is truncated

## Behavior

The program prints any changes to the file like `tail -f`, printing only happens when a newline is read.

Carriage returns are always ignored, this also means that the file can use either LF or CRLF endings.

It can be stopped with SIGINT (Ctrl-C), SIGTERM, or SIGHUP.

While running input echoing is disabled.

The file may contain ANSI escape sequences, these are are ignored for width calculation and are printed no matter what they are.

If the file is truncated the screen is cleared, if old content should be discarded the scrollback buffer is cleared as well.

Text is hard-wrapped using display width of Unicode characters, if timestamps are enabled text is wrapped to the width of the timestamps.

There is a status bar displaying the name of the file as well as the time the program was started or the file was truncated.

It does not react to changes in terminal size.
