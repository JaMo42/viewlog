#[cfg(target_family = "unix")]
mod detail {
    use std::os::unix::io::AsRawFd;
    use std::{
        io::{stdin, Result},
        mem::MaybeUninit,
    };
    use termios::{tcgetattr, tcsetattr, Termios, ECHO, TCSAFLUSH};

    pub type ConsoleMode = Termios;

    pub fn disable_echo() -> Result<ConsoleMode> {
        let fd = stdin().as_raw_fd();
        let mut old = unsafe { MaybeUninit::zeroed().assume_init() };
        tcgetattr(fd, &mut old)?;
        let mut new = old;
        new.c_lflag &= !ECHO;
        tcsetattr(fd, TCSAFLUSH, &new)?;
        Ok(old)
    }

    pub fn restore(mode: ConsoleMode) {
        tcsetattr(stdin().as_raw_fd(), TCSAFLUSH, &mode).ok();
    }
}

#[cfg(target_family = "windows")]
mod detail {
    use windows::{
        core::Result,
        Win32::System::Console::{
            GetConsoleMode, GetStdHandle, SetConsoleMode, CONSOLE_MODE, ENABLE_ECHO_INPUT,
            STD_INPUT_HANDLE,
        },
    };

    pub type ConsoleMode = CONSOLE_MODE;

    pub fn disable_echo() -> Result<ConsoleMode> {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE)?;
            let mut old = CONSOLE_MODE(0);
            GetConsoleMode(handle, &mut old).ok()?;
            let mut new = old;
            new &= !ENABLE_ECHO_INPUT;
            SetConsoleMode(handle, new).ok()?;
            Ok(old)
        }
    }

    pub fn restore(mode: ConsoleMode) {
        unsafe {
            if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
                SetConsoleMode(handle, mode);
            }
        }
    }
}

pub struct NoEcho {
    old_mode: Option<detail::ConsoleMode>,
}

impl NoEcho {
    /// Disable input echoing until the returned value is dropped.
    pub fn begin() -> Self {
        Self {
            old_mode: detail::disable_echo().ok(),
        }
    }
}

impl Drop for NoEcho {
    fn drop(&mut self) {
        if let Some(old_mode) = self.old_mode.take() {
            detail::restore(old_mode);
        }
    }
}
