//! Buffered stdin reader matching the VM's `getc` / `scanf` style input.

use crate::error::VmError;
use std::collections::VecDeque;
use std::io::{BufReader, Read};

pub struct Input<R: Read> {
    reader: BufReader<R>,
    pushback: VecDeque<u8>,
}

impl<R: Read> Input<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            pushback: VecDeque::new(),
        }
    }

    fn read_byte(&mut self) -> Result<Option<u8>, VmError> {
        if let Some(byte) = self.pushback.pop_front() {
            return Ok(Some(byte));
        }
        let mut buffer = [0u8; 1];
        match self.reader.read(&mut buffer) {
            Ok(0) => Ok(None),
            Ok(_) => Ok(Some(buffer[0])),
            Err(error) => Err(VmError::Io(error.to_string())),
        }
    }

    fn unread(&mut self, byte: u8) {
        self.pushback.push_front(byte);
    }

    /// `getch()` semantics: one byte, no line buffering, no echo.
    ///
    /// On a TTY this temporarily disables `ICANON|ECHO`, exactly like the
    /// original XASM1 `getch()`.  Pipes and files are unaffected.
    pub fn read_char(&mut self) -> Result<Option<u8>, VmError> {
        let _raw = crate::terminal::RawMode::enable();
        self.read_byte()
    }

    fn skip_whitespace(&mut self) -> Result<(), VmError> {
        while let Some(byte) = self.read_byte()? {
            if !byte.is_ascii_whitespace() {
                self.unread(byte);
                break;
            }
        }
        Ok(())
    }

    /// `scanf("%d")`-style integer parsing.
    pub fn read_decimal(&mut self) -> Result<Option<i32>, VmError> {
        self.skip_whitespace()?;
        let mut sign = 1i64;
        match self.read_byte()? {
            Some(b'-') => sign = -1,
            Some(b'+') => {}
            Some(byte) => self.unread(byte),
            None => return Ok(None),
        }
        let mut digits = Vec::new();
        while let Some(byte) = self.read_byte()? {
            if byte.is_ascii_digit() {
                digits.push(byte);
            } else {
                self.unread(byte);
                break;
            }
        }
        if digits.is_empty() {
            if sign < 0 {
                self.unread(b'-');
            }
            return Ok(None);
        }
        let mut value: i64 = 0;
        for digit in digits {
            value = value
                .saturating_mul(10)
                .saturating_add(i64::from(digit - b'0'));
        }
        Ok(Some((value * sign) as i32))
    }

    /// `scanf("%x")`-style hexadecimal parsing (an optional `0x` is accepted).
    pub fn read_hex(&mut self) -> Result<Option<i32>, VmError> {
        self.skip_whitespace()?;
        let mut sign = 1i64;
        match self.read_byte()? {
            Some(b'-') => sign = -1,
            Some(b'+') => {}
            Some(byte) => self.unread(byte),
            None => return Ok(None),
        }

        let mut digits = Vec::new();
        if let Some(byte) = self.read_byte()? {
            if byte == b'0' {
                if let Some(next) = self.read_byte()? {
                    if next == b'x' || next == b'X' {
                        // 0x prefix consumed
                    } else {
                        digits.push(byte);
                        self.unread(next);
                    }
                } else {
                    digits.push(byte);
                }
            } else {
                self.unread(byte);
            }
        }

        while let Some(byte) = self.read_byte()? {
            if byte.is_ascii_hexdigit() {
                digits.push(byte);
            } else {
                self.unread(byte);
                break;
            }
        }
        if digits.is_empty() {
            if sign < 0 {
                self.unread(b'-');
            }
            return Ok(None);
        }
        let mut value: i64 = 0;
        for digit in digits {
            let nibble = match digit {
                b'0'..=b'9' => i64::from(digit - b'0'),
                b'a'..=b'f' => i64::from(digit - b'a' + 10),
                b'A'..=b'F' => i64::from(digit - b'A' + 10),
                _ => unreachable!(),
            };
            value = value.saturating_mul(16).saturating_add(nibble);
        }
        Ok(Some((value * sign) as i32))
    }
}

impl<R: Read> std::fmt::Debug for Input<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input")
            .field("pushback", &self.pushback.len())
            .finish()
    }
}
