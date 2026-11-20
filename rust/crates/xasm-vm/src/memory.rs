//! Flat 32-bit cell memory with lazily allocated pages.
//!
//! The original XASM1 has a single flat array of `int` cells (`memory[0xffffe]`).
//! The community VM replaced it with sparse chunks.  A paged representation is
//! both faithful (cells are `i32`) and cheap for high addresses.

use crate::error::VmError;
use std::fmt;

pub const DEFAULT_MAX_ADDR: u32 = 0x000f_fffe;
pub const PAGE_BITS: u32 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;

pub struct Memory {
    pages: Vec<Option<Box<[i32; PAGE_SIZE]>>>,
    max_addr: u32,
    fill: i32,
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memory")
            .field("max_addr", &self.max_addr)
            .field(
                "allocated_pages",
                &self.pages.iter().filter(|p| p.is_some()).count(),
            )
            .finish()
    }
}

impl Memory {
    pub fn new(max_addr: u32) -> Self {
        Self::new_with_fill(max_addr, 0)
    }

    /// The original XASM1 loads every cell up to 0xffffe with `fgetc`.
    /// After EOF, `fgetc` returns -1, so all cells beyond the ROM are -1.
    pub fn new_legacy(max_addr: u32) -> Self {
        Self::new_with_fill(max_addr, -1)
    }

    pub fn new_with_fill(max_addr: u32, fill: i32) -> Self {
        let page_count = (max_addr >> PAGE_BITS) as usize + 1;
        Self {
            pages: (0..page_count).map(|_| None).collect(),
            max_addr,
            fill,
        }
    }

    pub fn fill(&self) -> i32 {
        self.fill
    }

    pub fn max_addr(&self) -> u32 {
        self.max_addr
    }

    pub fn contains(&self, addr: u32) -> bool {
        addr <= self.max_addr
    }

    fn check(&self, addr: u32) -> Result<(), VmError> {
        if addr > self.max_addr {
            Err(VmError::OutOfRange {
                addr,
                max: self.max_addr,
            })
        } else {
            Ok(())
        }
    }

    pub fn read(&self, addr: u32) -> Result<i32, VmError> {
        self.check(addr)?;
        let page = (addr >> PAGE_BITS) as usize;
        let offset = (addr & (PAGE_SIZE as u32 - 1)) as usize;
        Ok(match &self.pages[page] {
            Some(data) => data[offset],
            None => self.fill,
        })
    }

    pub fn write(&mut self, addr: u32, value: i32) -> Result<(), VmError> {
        self.check(addr)?;
        let page = (addr >> PAGE_BITS) as usize;
        let offset = (addr & (PAGE_SIZE as u32 - 1)) as usize;
        let fill = self.fill;
        let data = self.pages[page].get_or_insert_with(|| Box::new([fill; PAGE_SIZE]));
        data[offset] = value;
        Ok(())
    }

    /// Load a ROM image at address 0.  Cells after the image stay zero.
    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), VmError> {
        let capacity = self.max_addr as usize + 1;
        if rom.len() > capacity {
            return Err(VmError::RomTooLarge {
                size: rom.len(),
                capacity,
            });
        }
        for (index, byte) in rom.iter().enumerate() {
            self.write(index as u32, i32::from(*byte))?;
        }
        Ok(())
    }

    pub fn dump(&self, start: u32, end: u32) -> Result<String, VmError> {
        let end = end.min(self.max_addr);
        let mut out = String::new();
        let mut addr = start;
        while addr <= end {
            if addr % 0x10 == 0 {
                if addr != start {
                    out.push('\n');
                }
                out.push_str(&format!("0x{addr:08x}| "));
            }
            out.push_str(&format!("{:02x} ", self.read(addr)? as u8));
            addr = match addr.checked_add(1) {
                Some(next) => next,
                None => break,
            };
        }
        out.push('\n');
        Ok(out)
    }
}
