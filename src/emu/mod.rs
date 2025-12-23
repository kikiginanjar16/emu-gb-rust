// emulator module rootmod bus;
mod cart;
mod cpu;
mod ppu;

use anyhow::Result;

pub use bus::JoypadState;

pub struct Emulator {
    cpu: cpu::Cpu,
    bus: bus::Bus,
}

impl Emulator {
    pub fn new(rom_path: &str) -> Result<Self> {
        let cart = cart::Cartridge::load(rom_path)?;
        let bus = bus::Bus::new(cart);
        let cpu = cpu::Cpu::new();

        Ok(Self { cpu, bus })
    }

    pub fn run_frame(&mut self) {
        // DMG: ~70224 cycles per frame (approx)
        let mut cycles = 0u32;
        while cycles < 70224 {
            let c = self.cpu.step(&mut self.bus);
            self.bus.step(c);
            cycles += c as u32;
        }
    }

    pub fn framebuffer_rgba(&self) -> &[u8] {
        self.bus.ppu.framebuffer_rgba()
    }

    pub fn set_joypad(&mut self, s: JoypadState) {
        self.bus.joypad = s;
    }
}
