// bus placeholder
use super::{cart::Cartridge, ppu::Ppu};

#[derive(Default, Copy, Clone)]
pub struct JoypadState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
}

pub struct Bus {
    pub cart: Cartridge,
    pub wram: [u8; 0x2000], // C000-DFFF
    pub hram: [u8; 0x007F], // FF80-FFFE
    pub ppu: Ppu,
    pub joypad: JoypadState,

    // TODO: add timer, interrupt flags, IE, IF, etc.
    pub ie: u8, // FFFF
    pub iflag: u8, // FF0F
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            wram: [0; 0x2000],
            hram: [0; 0x007F],
            ppu: Ppu::new(),
            joypad: JoypadState::default(),
            ie: 0,
            iflag: 0,
        }
    }

    pub fn step(&mut self, cycles: u8) {
        self.ppu.step(cycles as u32);
        // TODO: timer/APU stepping + interrupts
    }

    pub fn read8(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.read(addr),          // ROM (no MBC yet)
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFF0F => self.iflag,
            0xFFFF => self.ie,
            _ => 0xFF, // TODO: map VRAM/OAM/IO
        }
    }

    pub fn write8(&mut self, addr: u16, v: u8) {
        match addr {
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = v,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = v,
            0xFF0F => self.iflag = v,
            0xFFFF => self.ie = v,
            _ => {
                // TODO: map VRAM/OAM/IO/MBC registers
            }
        }
    }
}
