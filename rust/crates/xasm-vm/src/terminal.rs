//! Terminal raw mode for `getc`.
//!
//! The original XASM1 `getch()` disables `ICANON|ECHO` around every character
//! read.  This module mirrors that behavior on Unix.  Pipes/files are left
//! alone; `isatty` fails and `enable` returns `None`.

#[cfg(unix)]
pub struct RawMode {
    original: libc::termios,
}

#[cfg(unix)]
impl RawMode {
    pub fn enable() -> Option<Self> {
        unsafe {
            if libc::isatty(libc::STDIN_FILENO) != 1 {
                return None;
            }
            let mut term: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(libc::STDIN_FILENO, &mut term) != 0 {
                return None;
            }
            let original = term;
            term.c_lflag &= !(libc::ICANON | libc::ECHO);
            term.c_cc[libc::VMIN] = 1;
            term.c_cc[libc::VTIME] = 0;
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &term) != 0 {
                return None;
            }
            Some(Self { original })
        }
    }
}

#[cfg(unix)]
impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original);
        }
    }
}

#[cfg(not(unix))]
pub struct RawMode;

#[cfg(not(unix))]
impl RawMode {
    pub fn enable() -> Option<Self> {
        None
    }
}
