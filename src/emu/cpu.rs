// cpu core placeholder
use super::bus::Bus;

#[derive(Default, Clone, Copy)]
struct Flags {
    z: bool, // zero
    n: bool, // subtract
    h: bool, // half-carry
    c: bool, // carry
}

pub struct Cpu {
    // 8-bit regs
    a: u8, f: Flags,
    b: u8, c: u8,
    d: u8, e: u8,
    h: u8, l: u8,

    sp: u16,
    pc: u16,

    ime: bool, // interrupt master enable
    halted: bool,
}

impl Cpu {
    pub fn new() -> Self {
        // NOTE: Real GB boot sequence sets registers after boot ROM.
        // For now we start at 0x0100 (where cartridge entry usually begins).
        Self {
            a: 0x01,
            f: Flags { z: true, n: false, h: true, c: true },
            b: 0x00, c: 0x13,
            d: 0x00, e: 0xD8,
            h: 0x01, l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
            halted: false,
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        if self.halted {
            return 4; // simplistic
        }

        let op = self.fetch8(bus);
        match op {
            0x00 => 4, // NOP

            0x3E => { // LD A, d8
                let v = self.fetch8(bus);
                self.a = v;
                8
            }

            0x06 => { // LD B, d8
                let v = self.fetch8(bus);
                self.b = v;
                8
            }

            0x0E => { // LD C, d8
                let v = self.fetch8(bus);
                self.c = v;
                8
            }

            0xAF => { // XOR A
                self.a ^= self.a;
                self.set_flags(self.a == 0, false, false, false);
                4
            }

            0xC3 => { // JP a16
                let addr = self.fetch16(bus);
                self.pc = addr;
                16
            }

            0xCD => { // CALL a16
                let addr = self.fetch16(bus);
                self.push16(bus, self.pc);
                self.pc = addr;
                24
            }

            0xC9 => { // RET
                let addr = self.pop16(bus);
                self.pc = addr;
                16
            }

            _ => {
                // TODO: implement the rest of opcodes
                // For now: stop hard so you see what opcode is missing.
                panic!("Unimplemented opcode: 0x{op:02X} at PC=0x{:04X}", self.pc.wrapping_sub(1));
            }
        }
    }

    fn fetch8(&mut self, bus: &mut Bus) -> u8 {
        let v = bus.read8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.fetch8(bus) as u16;
        let hi = self.fetch8(bus) as u16;
        (hi << 8) | lo
    }

    fn push16(&mut self, bus: &mut Bus, v: u16) {
        self.sp = self.sp.wrapping_sub(1);
        bus.write8(self.sp, (v >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        bus.write8(self.sp, (v & 0xFF) as u8);
    }

    fn pop16(&mut self, bus: &mut Bus) -> u16 {
        let lo = bus.read8(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let hi = bus.read8(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.f = Flags { z, n, h, c };
    }
}
