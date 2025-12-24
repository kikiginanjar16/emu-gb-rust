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
    pub vram: [u8; 0x2000], // 8000-9FFF
    pub oam: [u8; 0x00A0],  // FE00-FE9F
    pub ppu: Ppu,
    pub joypad: JoypadState,
    joyp_select: u8, // bits 4/5 of P1

    // TODO: add timer, interrupt flags, IE, IF, etc.
    pub ie: u8, // FFFF
    pub iflag: u8, // FF0F
    // Timers
    div: u16,
    tima: u8,
    tma: u8,
    tac: u8,
    timer_counter: u32,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        Self {
            cart,
            wram: [0; 0x2000],
            hram: [0; 0x007F],
            vram: [0; 0x2000],
            oam: [0; 0x00A0],
            ppu: Ppu::new(),
            joypad: JoypadState::default(),
            joyp_select: 0x00,
            ie: 0,
            iflag: 0,
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            timer_counter: 0,
        }
    }

    pub fn step(&mut self, cycles: u8) {
        let vram = &self.vram;
        let (vblank, stat_irq) = self.ppu.step(cycles as u32, vram);
        if vblank {
            // Set VBlank interrupt
            self.iflag |= 0x01;
        }
        if stat_irq {
            self.iflag |= 0x02;
        }
        self.tick_timer(cycles as u32);
        // TODO: APU stepping
    }

    pub fn read8(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.read(addr),          // ROM (no MBC yet)
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize], // echo RAM
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            0xFF00 => self.read_joyp(),
            0xFF40 => self.ppu.lcdc,
            0xFF41 => self.ppu.stat,
            0xFF42 => self.ppu.scy,
            0xFF43 => self.ppu.scx,
            0xFF44 => self.ppu.ly,
            0xFF45 => self.ppu.lyc,
            0xFF47 => self.ppu.bgp,
            0xFF4A => self.ppu.wy,
            0xFF4B => self.ppu.wx,
            0xFF04 => (self.div >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFF0F => self.iflag,
            0xFFFF => self.ie,
            _ => 0xFF, // TODO: map VRAM/OAM/IO
        }
    }

    pub fn write8(&mut self, addr: u16, v: u8) {
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = v,
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = v,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = v, // echo RAM
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = v,
            0xFF00 => self.joyp_select = v & 0x30,
            0xFF40 => self.ppu.lcdc = v,
            0xFF41 => self.ppu.stat = (self.ppu.stat & 0x07) | (v & 0x78), // only interrupt bits writable
            0xFF42 => self.ppu.scy = v,
            0xFF43 => self.ppu.scx = v,
            0xFF44 => self.ppu.ly = 0, // writing resets LY
            0xFF45 => self.ppu.lyc = v,
            0xFF47 => self.ppu.bgp = v,
            0xFF4A => self.ppu.wy = v,
            0xFF4B => self.ppu.wx = v,
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = v,
            0xFF06 => self.tma = v,
            0xFF07 => self.tac = v & 0x07,
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = v,
            0xFF0F => self.iflag = v,
            0xFFFF => self.ie = v,
            _ => {
                if addr == 0xFF46 {
                    let base = (v as u16) << 8;
                    for i in 0..0xA0 {
                        let data = self.read8(base + i);
                        self.oam[i as usize] = data;
                    }
                }
                // TODO: map VRAM/OAM/IO/MBC registers
            }
        }
    }

    fn read_joyp(&self) -> u8 {
        // Bit = 0 means selected/pressed. Unselected lines return 1.
        let mut res = 0xCF | (self.joyp_select & 0x30);
        let sel_dpad = self.joyp_select & 0x10 == 0;
        let sel_btn = self.joyp_select & 0x20 == 0;

        if sel_dpad {
            if self.joypad.right { res &= !0x01; }
            if self.joypad.left { res &= !0x02; }
            if self.joypad.up { res &= !0x04; }
            if self.joypad.down { res &= !0x08; }
        }

        if sel_btn {
            if self.joypad.a { res &= !0x01; }
            if self.joypad.b { res &= !0x02; }
            if self.joypad.select { res &= !0x04; }
            if self.joypad.start { res &= !0x08; }
        }

        res
    }

    fn timer_freq_divider(&self) -> u32 {
        match self.tac & 0x03 {
            0 => 1024, // 4096 Hz
            1 => 16,   // 262144 Hz
            2 => 64,   // 65536 Hz
            _ => 256,  // 16384 Hz
        }
    }

    fn tick_timer(&mut self, cycles: u32) {
        // DIV increments at 16384 Hz: +4 per CPU cycle on the upper byte.
        self.div = self.div.wrapping_add((cycles * 4) as u16);

        if self.tac & 0x04 == 0 {
            return;
        }

        self.timer_counter += cycles;
        let period = self.timer_freq_divider();
        while self.timer_counter >= period {
            self.timer_counter -= period;
            let (new, overflow) = self.tima.overflowing_add(1);
            if overflow {
                self.tima = self.tma;
                self.iflag |= 0x04; // timer interrupt
            } else {
                self.tima = new;
            }
        }
    }
}
