// cartridge loader placeholder
use anyhow::{bail, Result};
use std::fs;

#[derive(Clone)]
pub struct Cartridge {
    pub rom: Vec<u8>,
}

impl Cartridge {
    pub fn load(path: &str) -> Result<Self> {
        let rom = fs::read(path)?;
        if rom.len() < 0x150 {
            bail!("ROM too small / invalid");
        }
        Ok(Self { rom })
    }

    pub fn read(&self, addr: u16) -> u8 {
        let i = addr as usize;
        if i < self.rom.len() { self.rom[i] } else { 0xFF }
    }
}
