// cpu core placeholder
use super::bus::Bus;

#[derive(Default, Clone, Copy)]
struct Flags {
    z: bool, // zero
    n: bool, // subtract
    h: bool, // half-carry
    c: bool, // carry
}

impl Flags {
    fn to_byte(self) -> u8 {
        (self.z as u8) << 7
            | (self.n as u8) << 6
            | (self.h as u8) << 5
            | (self.c as u8) << 4
    }

    fn from_byte(v: u8) -> Self {
        Self {
            z: v & 0x80 != 0,
            n: v & 0x40 != 0,
            h: v & 0x20 != 0,
            c: v & 0x10 != 0,
        }
    }
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
        // Simple interrupt handling (VBlank only for now)
        let pending = bus.ie & bus.iflag;
        if pending != 0 {
            if self.ime {
                self.halted = false;
                let bit = pending.trailing_zeros() as u16;
                bus.iflag &= !(1 << bit);
                self.ime = false;
                self.push16(bus, self.pc);
                self.pc = 0x40 + bit * 8;
                return 20;
            } else if self.halted {
                self.halted = false;
            }
        }

        if self.halted {
            return 4; // simplistic
        }

        let op = self.fetch8(bus);
        match op {
            0x00 => 4, // NOP

            0x01 => { // LD BC, d16
                let v = self.fetch16(bus);
                self.set_bc(v);
                12
            }

            0x02 => { // LD (BC), A
                bus.write8(self.bc(), self.a);
                8
            }

            0x03 => { // INC BC
                let v = self.bc().wrapping_add(1);
                self.set_bc(v);
                8
            }

            0x04 => { // INC B
                self.b = self.inc8(self.b);
                4
            }

            0x05 => {
                self.b = self.dec8(self.b);
                4
            }

            0x06 => { // LD B, d8
                let v = self.fetch8(bus);
                self.b = v;
                8
            }

            0x07 => { // RLCA
                let carry = self.a & 0x80 != 0;
                self.a = self.a.rotate_left(1);
                self.f = Flags { z: false, n: false, h: false, c: carry };
                4
            }

            0x08 => { // LD (a16), SP
                let addr = self.fetch16(bus);
                bus.write8(addr, (self.sp & 0xFF) as u8);
                bus.write8(addr.wrapping_add(1), (self.sp >> 8) as u8);
                20
            }

            0x09 => { // ADD HL, BC
                let hl = self.hl();
                let bc = self.bc();
                let res = hl.wrapping_add(bc);
                self.f.n = false;
                self.f.h = ((hl & 0x0FFF) + (bc & 0x0FFF)) > 0x0FFF;
                self.f.c = hl > 0xFFFF - bc;
                self.set_hl(res);
                8
            }

            0x0A => { // LD A, (BC)
                self.a = bus.read8(self.bc());
                8
            }

            0x0B => { // DEC BC
                let v = self.bc().wrapping_sub(1);
                self.set_bc(v);
                8
            }

            0x0C => { // INC C
                let v = self.c;
                let res = v.wrapping_add(1);
                self.f.z = res == 0;
                self.f.n = false;
                self.f.h = (v & 0x0F) == 0x0F;
                self.c = res;
                4
            }

            0x0D => { // DEC C
                let v = self.c;
                let res = v.wrapping_sub(1);
                self.f.z = res == 0;
                self.f.n = true;
                self.f.h = (v & 0x0F) == 0;
                self.c = res;
                4
            }

            0x0E => { // LD C, d8
                let v = self.fetch8(bus);
                self.c = v;
                8
            }

            0x0F => { // RRCA
                let carry = self.a & 1 != 0;
                self.a = self.a.rotate_right(1);
                self.f = Flags { z: false, n: false, h: false, c: carry };
                4
            }

            0x10 => { // STOP (treated as NOP)
                4
            }

            0x11 => { // LD DE, d16
                let v = self.fetch16(bus);
                self.set_de(v);
                12
            }

            0x12 => { // LD (DE), A
                bus.write8(self.de(), self.a);
                8
            }

            0x13 => { // INC DE
                let v = self.de().wrapping_add(1);
                self.set_de(v);
                8
            }

            0x14 => { // INC D
                self.d = self.inc8(self.d);
                4
            }

            0x15 => { // DEC D
                self.d = self.dec8(self.d);
                4
            }

            0x16 => { // LD D, d8
                self.d = self.fetch8(bus);
                8
            }

            0x17 => { // RLA
                let new_c = self.a & 0x80 != 0;
                let carry = self.f.c;
                self.a = (self.a << 1) | (carry as u8);
                self.f = Flags { z: false, n: false, h: false, c: new_c };
                4
            }

            0x18 => { // JR r8
                let off = self.fetch8(bus) as i8;
                self.pc = self.pc.wrapping_add(off as u16);
                12
            }

            0x19 => { // ADD HL, DE
                let hl = self.hl();
                let de = self.de();
                let res = hl.wrapping_add(de);
                self.f.n = false;
                self.f.h = ((hl & 0x0FFF) + (de & 0x0FFF)) > 0x0FFF;
                self.f.c = hl > 0xFFFF - de;
                self.set_hl(res);
                8
            }

            0x1A => { // LD A, (DE)
                self.a = bus.read8(self.de());
                8
            }

            0x1B => { // DEC DE
                let v = self.de().wrapping_sub(1);
                self.set_de(v);
                8
            }

            0x1C => { // INC E
                self.e = self.inc8(self.e);
                4
            }

            0x1D => { // DEC E
                self.e = self.dec8(self.e);
                4
            }

            0x1E => { // LD E, d8
                self.e = self.fetch8(bus);
                8
            }

            0x1F => { // RRA
                let new_c = self.a & 1 != 0;
                let carry = self.f.c;
                self.a = (self.a >> 1) | ((carry as u8) << 7);
                self.f = Flags { z: false, n: false, h: false, c: new_c };
                4
            }

            0x20 => { // JR NZ, r8
                let off = self.fetch8(bus) as i8;
                if !self.f.z {
                    self.pc = self.pc.wrapping_add(off as u16);
                    12
                } else {
                    8
                }
            }

            0x21 => { // LD HL, d16
                let v = self.fetch16(bus);
                self.set_hl(v);
                12
            }

            0x22 => { // LD (HL+), A
                let addr = self.hl();
                bus.write8(addr, self.a);
                self.set_hl(addr.wrapping_add(1));
                8
            }

            0x23 => { // INC HL
                let v = self.hl().wrapping_add(1);
                self.set_hl(v);
                8
            }

            0x28 => { // JR Z, r8
                let off = self.fetch8(bus) as i8;
                if self.f.z {
                    self.pc = self.pc.wrapping_add(off as u16);
                    12
                } else {
                    8
                }
            }

            0x2A => { // LD A, (HL+)
                let addr = self.hl();
                self.a = bus.read8(addr);
                self.set_hl(addr.wrapping_add(1));
                8
            }

            0x2B => { // DEC HL
                let v = self.hl().wrapping_sub(1);
                self.set_hl(v);
                8
            }

            0x24 => { // INC H
                self.h = self.inc8(self.h);
                4
            }

            0x25 => { // DEC H
                self.h = self.dec8(self.h);
                4
            }

            0x26 => { // LD H, d8
                self.h = self.fetch8(bus);
                8
            }

            0x27 => { // DAA (approx)
                let mut a = self.a as i16;
                if !self.f.n {
                    if self.f.h || (a & 0x0F) > 9 {
                        a += 0x06;
                    }
                    if self.f.c || a > 0x9F {
                        a += 0x60;
                        self.f.c = true;
                    }
                } else {
                    if self.f.h {
                        a = (a - 0x06) & 0xFF;
                    }
                    if self.f.c {
                        a -= 0x60;
                    }
                }
                self.a = (a & 0xFF) as u8;
                self.f.z = self.a == 0;
                self.f.h = false;
                4
            }

            0x29 => { // ADD HL, HL
                let hl = self.hl();
                let res = hl.wrapping_add(hl);
                self.f.n = false;
                self.f.h = ((hl & 0x0FFF) + (hl & 0x0FFF)) > 0x0FFF;
                self.f.c = hl > 0x7FFF;
                self.set_hl(res);
                8
            }

            0x2C => { // INC L
                let v = self.l;
                let res = v.wrapping_add(1);
                self.f.z = res == 0;
                self.f.n = false;
                self.f.h = (v & 0x0F) == 0x0F;
                self.l = res;
                4
            }

            0x2D => { // DEC L
                self.l = self.dec8(self.l);
                4
            }

            0x2E => { // LD L, d8
                self.l = self.fetch8(bus);
                8
            }

            0x2F => { // CPL
                self.a = !self.a;
                self.f.n = true;
                self.f.h = true;
                4
            }

            0x31 => { // LD SP, d16
                let v = self.fetch16(bus);
                self.sp = v;
                12
            }

            0x30 => { // JR NC, r8
                let off = self.fetch8(bus) as i8;
                if !self.f.c {
                    self.pc = self.pc.wrapping_add(off as u16);
                    12
                } else {
                    8
                }
            }

            0x32 => { // LD (HL-), A
                let addr = self.hl();
                bus.write8(addr, self.a);
                self.set_hl(addr.wrapping_sub(1));
                8
            }

            0x33 => { // INC SP
                self.sp = self.sp.wrapping_add(1);
                8
            }

            0x34 => { // INC (HL)
                let addr = self.hl();
                let v = bus.read8(addr);
                let res = v.wrapping_add(1);
                bus.write8(addr, res);
                self.f.z = res == 0;
                self.f.n = false;
                self.f.h = (v & 0x0F) == 0x0F;
                12
            }

            0x35 => { // DEC (HL)
                let addr = self.hl();
                let v = bus.read8(addr);
                let res = v.wrapping_sub(1);
                bus.write8(addr, res);
                self.f.z = res == 0;
                self.f.n = true;
                self.f.h = (v & 0x0F) == 0;
                12
            }

            0x36 => { // LD (HL), d8
                let v = self.fetch8(bus);
                let addr = self.hl();
                bus.write8(addr, v);
                12
            }

            0x38 => { // JR C, r8
                let off = self.fetch8(bus) as i8;
                if self.f.c {
                    self.pc = self.pc.wrapping_add(off as u16);
                    12
                } else {
                    8
                }
            }

            0x39 => { // ADD HL, SP
                let hl = self.hl();
                let sp = self.sp;
                let res = hl.wrapping_add(sp);
                self.f.n = false;
                self.f.h = ((hl & 0x0FFF) + (sp & 0x0FFF)) > 0x0FFF;
                self.f.c = hl > 0xFFFF - sp;
                self.set_hl(res);
                8
            }

            0x3A => { // LD A, (HL-)
                let addr = self.hl();
                self.a = bus.read8(addr);
                self.set_hl(addr.wrapping_sub(1));
                8
            }

            0x3B => { // DEC SP
                self.sp = self.sp.wrapping_sub(1);
                8
            }

            0x3C => { // INC A
                let v = self.a;
                let res = v.wrapping_add(1);
                self.f.z = res == 0;
                self.f.n = false;
                self.f.h = (v & 0x0F) == 0x0F;
                self.a = res;
                4
            }

            0x3D => { // DEC A
                self.a = self.dec8(self.a);
                4
            }

            0x3E => { // LD A, d8
                let v = self.fetch8(bus);
                self.a = v;
                8
            }

            0x3F => { // CCF
                self.f.c = !self.f.c;
                self.f.n = false;
                self.f.h = false;
                4
            }

            0x37 => { // SCF
                self.f.c = true;
                self.f.n = false;
                self.f.h = false;
                4
            }

            0x40..=0x7F => {
                if op == 0x76 {
                    self.halted = true;
                    return 4;
                }
                let dst = ((op >> 3) & 0x07) as usize;
                let src = (op & 0x07) as usize;
                let val = if src == 6 {
                    bus.read8(self.hl())
                } else {
                    self.get_reg(src)
                };
                if dst == 6 {
                    bus.write8(self.hl(), val);
                } else {
                    self.set_reg(dst, val);
                }
                4 + if src == 6 || dst == 6 { 4 } else { 0 }
            }

            0x80..=0x87 => { // ADD A, r
                let v = if op == 0x86 { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.add_a(v);
                4 + if op == 0x86 { 4 } else { 0 }
            }

            0x88..=0x8F => { // ADC A, r
                let v = if op == 0x8E { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.adc_a(v);
                4 + if op == 0x8E { 4 } else { 0 }
            }

            0x90..=0x97 => { // SUB r
                let v = if op == 0x96 { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.sub_a(v);
                4 + if op == 0x96 { 4 } else { 0 }
            }

            0x98..=0x9F => { // SBC r
                let v = if op == 0x9E { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.sbc_a(v);
                4 + if op == 0x9E { 4 } else { 0 }
            }

            0xA0..=0xA7 => { // AND r
                let v = if op == 0xA6 { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.and_a(v);
                4 + if op == 0xA6 { 4 } else { 0 }
            }

            0xA8..=0xAF => { // XOR r
                let v = if op == 0xAE { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.xor_a(v);
                4 + if op == 0xAE { 4 } else { 0 }
            }

            0xB0..=0xB7 => { // OR r
                let v = if op == 0xB6 { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.or_a(v);
                4 + if op == 0xB6 { 4 } else { 0 }
            }

            0xB8..=0xBF => { // CP r
                let v = if op == 0xBE { bus.read8(self.hl()) } else { self.get_reg((op & 0x07) as usize) };
                self.cp_a(v);
                4 + if op == 0xBE { 4 } else { 0 }
            }

            0xC1 => { // POP BC
                let v = self.pop16(bus);
                self.set_bc(v);
                12
            }

            0xC0 => { // RET NZ
                if !self.f.z {
                    let addr = self.pop16(bus);
                    self.pc = addr;
                    20
                } else {
                    8
                }
            }

            0xC3 => { // JP a16
                let addr = self.fetch16(bus);
                self.pc = addr;
                16
            }

            0xC2 => { // JP NZ, a16
                let addr = self.fetch16(bus);
                if !self.f.z {
                    self.pc = addr;
                    16
                } else {
                    12
                }
            }

            0xC5 => { // PUSH BC
                self.push16(bus, self.bc());
                16
            }

            0xC6 => { // ADD A, d8
                let v = self.fetch8(bus);
                self.add_a(v);
                8
            }

            0xC7 => { // RST 00h
                self.push16(bus, self.pc);
                self.pc = 0x00;
                16
            }

            0xC8 => { // RET Z
                if self.f.z {
                    let addr = self.pop16(bus);
                    self.pc = addr;
                    20
                } else {
                    8
                }
            }

            0xC9 => { // RET
                let addr = self.pop16(bus);
                self.pc = addr;
                16
            }

            0xCE => { // ADC A, d8
                let v = self.fetch8(bus);
                self.adc_a(v);
                8
            }

            0xCA => { // JP Z, a16
                let addr = self.fetch16(bus);
                if self.f.z {
                    self.pc = addr;
                    16
                } else {
                    12
                }
            }

            0xCF => { // RST 08h
                self.push16(bus, self.pc);
                self.pc = 0x08;
                16
            }

            0xCB => {
                let cb = self.fetch8(bus);
                return self.cb_op(cb, bus);
            }

            0xD2 => { // JP NC, a16
                let addr = self.fetch16(bus);
                if !self.f.c {
                    self.pc = addr;
                    16
                } else {
                    12
                }
            }

            0xDA => { // JP C, a16
                let addr = self.fetch16(bus);
                if self.f.c {
                    self.pc = addr;
                    16
                } else {
                    12
                }
            }

            0xCC => { // CALL Z, a16
                let addr = self.fetch16(bus);
                if self.f.z {
                    self.push16(bus, self.pc);
                    self.pc = addr;
                    24
                } else {
                    12
                }
            }

            0xC4 => { // CALL NZ, a16
                let addr = self.fetch16(bus);
                if !self.f.z {
                    self.push16(bus, self.pc);
                    self.pc = addr;
                    24
                } else {
                    12
                }
            }

            0xCD => { // CALL a16
                let addr = self.fetch16(bus);
                self.push16(bus, self.pc);
                self.pc = addr;
                24
            }

            0xD1 => { // POP DE
                let v = self.pop16(bus);
                self.set_de(v);
                12
            }

            0xD0 => { // RET NC
                if !self.f.c {
                    let addr = self.pop16(bus);
                    self.pc = addr;
                    20
                } else {
                    8
                }
            }

            0xD5 => { // PUSH DE
                self.push16(bus, self.de());
                16
            }

            0xD6 => { // SUB d8
                let v = self.fetch8(bus);
                self.sub_a(v);
                8
            }

            0xD7 => { // RST 10h
                self.push16(bus, self.pc);
                self.pc = 0x10;
                16
            }

            0xD9 => { // RETI
                let addr = self.pop16(bus);
                self.ime = true;
                self.pc = addr;
                16
            }

            0xD8 => { // RET C
                if self.f.c {
                    let addr = self.pop16(bus);
                    self.pc = addr;
                    20
                } else {
                    8
                }
            }

            0xDE => { // SBC d8
                let v = self.fetch8(bus);
                self.sbc_a(v);
                8
            }

            0xDF => { // RST 18h
                self.push16(bus, self.pc);
                self.pc = 0x18;
                16
            }

            0xD4 => { // CALL NC, a16
                let addr = self.fetch16(bus);
                if !self.f.c {
                    self.push16(bus, self.pc);
                    self.pc = addr;
                    24
                } else {
                    12
                }
            }

            0xDC => { // CALL C, a16
                let addr = self.fetch16(bus);
                if self.f.c {
                    self.push16(bus, self.pc);
                    self.pc = addr;
                    24
                } else {
                    12
                }
            }

            0xE0 => { // LDH (a8), A
                let n = self.fetch8(bus);
                let addr = 0xFF00 | n as u16;
                bus.write8(addr, self.a);
                12
            }

            0xE1 => { // POP HL
                let v = self.pop16(bus);
                self.set_hl(v);
                12
            }

            0xE2 => { // LD (C), A
                let addr = 0xFF00 | self.c as u16;
                bus.write8(addr, self.a);
                8
            }

            0xE5 => { // PUSH HL
                self.push16(bus, self.hl());
                16
            }

            0xE6 => { // AND d8
                let v = self.fetch8(bus);
                self.a &= v;
                self.f = Flags { z: self.a == 0, n: false, h: true, c: false };
                8
            }

            0xEE => { // XOR d8
                let v = self.fetch8(bus);
                self.xor_a(v);
                8
            }

            0xE8 => { // ADD SP, r8
                let n = self.fetch8(bus) as i8 as i16 as u16;
                let sp = self.sp;
                let res = sp.wrapping_add(n);
                self.f.z = false;
                self.f.n = false;
                self.f.h = (sp & 0x0F) + (n & 0x0F) > 0x0F;
                self.f.c = (sp & 0xFF) + (n & 0xFF) > 0xFF;
                self.sp = res;
                16
            }

            0xEA => { // LD (a16), A
                let addr = self.fetch16(bus);
                bus.write8(addr, self.a);
                16
            }

            0xEF => { // RST 28h
                self.push16(bus, self.pc);
                self.pc = 0x28;
                16
            }

            0xE9 => { // JP (HL)
                self.pc = self.hl();
                4
            }

            0xE7 => { // RST 20h
                self.push16(bus, self.pc);
                self.pc = 0x20;
                16
            }

            0xF0 => { // LDH A, (a8)
                let n = self.fetch8(bus);
                let addr = 0xFF00 | n as u16;
                self.a = bus.read8(addr);
                12
            }

            0xF1 => { // POP AF
                let v = self.pop16(bus);
                self.a = (v >> 8) as u8;
                self.f = Flags::from_byte((v & 0xF0) as u8);
                12
            }

            0xF2 => { // LD A, (C)
                let addr = 0xFF00 | self.c as u16;
                self.a = bus.read8(addr);
                8
            }

            0xF3 => { // DI
                self.ime = false;
                4
            }

            0xF7 => { // RST 30h
                self.push16(bus, self.pc);
                self.pc = 0x30;
                16
            }

            0xF8 => { // LD HL, SP+r8
                let n = self.fetch8(bus) as i8 as i16 as u16;
                let res = self.sp.wrapping_add(n);
                self.f.z = false;
                self.f.n = false;
                self.f.h = (self.sp & 0x0F) + (n & 0x0F) > 0x0F;
                self.f.c = (self.sp & 0xFF) + (n & 0xFF) > 0xFF;
                self.set_hl(res);
                12
            }

            0xF9 => { // LD SP, HL
                self.sp = self.hl();
                8
            }

            0xF5 => { // PUSH AF
                let v = ((self.a as u16) << 8) | self.f.to_byte() as u16;
                self.push16(bus, v);
                16
            }

            0xF6 => { // OR d8
                let v = self.fetch8(bus);
                self.a |= v;
                self.f = Flags { z: self.a == 0, n: false, h: false, c: false };
                8
            }

            0xFA => { // LD A, (a16)
                let addr = self.fetch16(bus);
                self.a = bus.read8(addr);
                16
            }

            0xFB => { // EI
                self.ime = true;
                4
            }

            0xFF => { // RST 38h
                self.push16(bus, self.pc);
                self.pc = 0x38;
                16
            }

            0xFE => { // CP d8
                let v = self.fetch8(bus);
                let res = self.a.wrapping_sub(v);
                self.f.z = res == 0;
                self.f.n = true;
                self.f.h = (self.a & 0x0F) < (v & 0x0F);
                self.f.c = self.a < v;
                8
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

    fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }

    fn set_bc(&mut self, v: u16) {
        self.b = (v >> 8) as u8;
        self.c = v as u8;
    }

    fn de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }

    fn set_de(&mut self, v: u16) {
        self.d = (v >> 8) as u8;
        self.e = v as u8;
    }

    fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }

    fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    fn inc8(&mut self, v: u8) -> u8 {
        let res = v.wrapping_add(1);
        self.f.z = res == 0;
        self.f.n = false;
        self.f.h = (v & 0x0F) == 0x0F;
        res
    }

    fn dec8(&mut self, v: u8) -> u8 {
        let res = v.wrapping_sub(1);
        self.f.z = res == 0;
        self.f.n = true;
        self.f.h = (v & 0x0F) == 0;
        res
    }

    fn add_a(&mut self, v: u8) {
        let (res, carry) = self.a.overflowing_add(v);
        self.f.z = res == 0;
        self.f.n = false;
        self.f.h = (self.a & 0x0F) + (v & 0x0F) > 0x0F;
        self.f.c = carry;
        self.a = res;
    }

    fn sub_a(&mut self, v: u8) {
        let (res, borrow) = self.a.overflowing_sub(v);
        self.f.z = res == 0;
        self.f.n = true;
        self.f.h = (self.a & 0x0F) < (v & 0x0F);
        self.f.c = borrow;
        self.a = res;
    }

    fn adc_a(&mut self, v: u8) {
        let c = self.f.c as u8;
        let (t, c1) = self.a.overflowing_add(v);
        let (res, c2) = t.overflowing_add(c);
        self.f.z = res == 0;
        self.f.n = false;
        self.f.h = (self.a & 0x0F) + (v & 0x0F) + c > 0x0F;
        self.f.c = c1 || c2;
        self.a = res;
    }

    fn sbc_a(&mut self, v: u8) {
        let c = self.f.c as u8;
        let (t, b1) = self.a.overflowing_sub(v);
        let (res, b2) = t.overflowing_sub(c);
        self.f.z = res == 0;
        self.f.n = true;
        self.f.h = (self.a & 0x0F) < (v & 0x0F) + c;
        self.f.c = b1 || b2;
        self.a = res;
    }

    fn and_a(&mut self, v: u8) {
        self.a &= v;
        self.f = Flags { z: self.a == 0, n: false, h: true, c: false };
    }

    fn xor_a(&mut self, v: u8) {
        self.a ^= v;
        self.f = Flags { z: self.a == 0, n: false, h: false, c: false };
    }

    fn or_a(&mut self, v: u8) {
        self.a |= v;
        self.f = Flags { z: self.a == 0, n: false, h: false, c: false };
    }

    fn cp_a(&mut self, v: u8) {
        let res = self.a.wrapping_sub(v);
        self.f.z = res == 0;
        self.f.n = true;
        self.f.h = (self.a & 0x0F) < (v & 0x0F);
        self.f.c = self.a < v;
    }

    fn cb_op(&mut self, op: u8, bus: &mut Bus) -> u8 {
        let target = (op & 0x07) as usize;
        let bit = (op >> 3) & 0x07;
        let group = op >> 6;

        let mut val = if target == 6 { bus.read8(self.hl()) } else { self.get_reg(target) };
        let cycles = if target == 6 { 16 } else { 8 };

        match group {
            0 => { // rotates/shifts/swap
                match bit {
                    0 => { // RLC
                        let carry = val & 0x80 != 0;
                        val = val.rotate_left(1);
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                    1 => { // RRC
                        let carry = val & 1 != 0;
                        val = val.rotate_right(1);
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                    2 => { // RL
                        let carry = val & 0x80 != 0;
                        val = (val << 1) | (self.f.c as u8);
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                    3 => { // RR
                        let carry = val & 1 != 0;
                        val = (val >> 1) | ((self.f.c as u8) << 7);
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                    4 => { // SLA
                        let carry = val & 0x80 != 0;
                        val <<= 1;
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                    5 => { // SRA
                        let carry = val & 1 != 0;
                        val = (val >> 1) | (val & 0x80);
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                    6 => { // SWAP
                        val = (val >> 4) | (val << 4);
                        self.f = Flags { z: val == 0, n: false, h: false, c: false };
                    }
                    _ => { // SRL
                        let carry = val & 1 != 0;
                        val >>= 1;
                        self.f = Flags { z: val == 0, n: false, h: false, c: carry };
                    }
                }

                if target == 6 { bus.write8(self.hl(), val); } else { self.set_reg(target, val); }
            }
            1 => { // BIT
                let mask = 1 << bit;
                self.f.z = val & mask == 0;
                self.f.n = false;
                self.f.h = true;
            }
            2 => { // RES
                val &= !(1 << bit);
                if target == 6 { bus.write8(self.hl(), val); } else { self.set_reg(target, val); }
            }
            _ => { // SET
                val |= 1 << bit;
                if target == 6 { bus.write8(self.hl(), val); } else { self.set_reg(target, val); }
            }
        }

        cycles
    }

    fn get_reg(&self, idx: usize) -> u8 {
        match idx {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => unreachable!(),
            _ => self.a,
        }
    }

    fn set_reg(&mut self, idx: usize, v: u8) {
        match idx {
            0 => self.b = v,
            1 => self.c = v,
            2 => self.d = v,
            3 => self.e = v,
            4 => self.h = v,
            5 => self.l = v,
            6 => unreachable!(),
            _ => self.a = v,
        }
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

}
