// ppu placeholder
pub struct Ppu {
    fb: Vec<u8>, // RGBA 160*144*4
}

impl Ppu {
    pub fn new() -> Self {
        Self { fb: vec![0; 160 * 144 * 4] }
    }

    pub fn step(&mut self, _cycles: u32) {
        // TODO: implement PPU modes + render BG/window/sprites into fb
        // For now, show a simple “checker” so you know it runs.
        // You can remove this once PPU is implemented.
        for y in 0..144 {
            for x in 0..160 {
                let idx = (y * 160 + x) * 4;
                let on = ((x / 8) ^ (y / 8)) & 1 == 1;
                let v = if on { 0xCC } else { 0x33 };
                self.fb[idx] = v;
                self.fb[idx + 1] = v;
                self.fb[idx + 2] = v;
                self.fb[idx + 3] = 0xFF;
            }
        }
    }

    pub fn framebuffer_rgba(&self) -> &[u8] {
        &self.fb
    }
}
