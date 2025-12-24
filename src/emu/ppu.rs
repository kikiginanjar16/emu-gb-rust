const SCREEN_W: usize = 160;
const SCREEN_H: usize = 144;

pub struct Ppu {
    pub fb: Vec<u8>, // RGBA 160*144*4
    pub lcdc: u8,
    pub stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub bgp: u8,
    pub wy: u8,
    pub wx: u8,
    cycle_acc: u32,
    mode: u8,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            fb: vec![0; SCREEN_W * SCREEN_H * 4],
            lcdc: 0x91, // LCDC default after BIOS
            stat: 0x85, // mode 1, LY==LYC false
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0xFC, // default: 00->white, 11->black
            wy: 0,
            wx: 0,
            cycle_acc: 0,
            mode: 1,
        }
    }

    pub fn step(&mut self, cycles: u32, vram: &[u8]) -> (bool, bool) {
        if self.lcdc & 0x80 == 0 {
            self.ly = 0;
            self.fb.fill(0xFF);
            self.mode = 0;
            self.update_stat(false);
            return (false, false);
        }

        self.cycle_acc += cycles;
        let mut vblank = false;
        let mut stat_irq = false;

        while self.cycle_acc >= 456 {
            self.cycle_acc -= 456;
            self.ly = self.ly.wrapping_add(1);

            if self.ly == 144 {
                vblank = true;
            } else if self.ly >= 154 {
                self.ly = 0;
                self.render_background(vram);
            }
        }

        let mode = if self.ly >= 144 {
            1
        } else if self.cycle_acc < 80 {
            2
        } else if self.cycle_acc < 252 {
            3
        } else {
            0
        };
        let mode_changed = mode != self.mode;
        self.mode = mode;

        if self.update_stat(vblank || mode_changed) {
            stat_irq = true;
        }

        (vblank, stat_irq)
    }

    fn render_background(&mut self, vram: &[u8]) {
        if self.lcdc & 0x80 == 0 {
            self.fb.fill(0xFF);
            return;
        }

        let bg_tile_map_base = if self.lcdc & 0x08 != 0 { 0x9C00 } else { 0x9800 };
        let use_signed = self.lcdc & 0x10 == 0;

        for y in 0..SCREEN_H {
            let sy = y.wrapping_add(self.scy as usize) & 0xFF;
            for x in 0..SCREEN_W {
                let sx = x.wrapping_add(self.scx as usize) & 0xFF;
                let tile_x = sx / 8;
                let tile_y = sy / 8;
                let tile_map_index = tile_y * 32 + tile_x;
                let tile_num = vram[(bg_tile_map_base - 0x8000 + tile_map_index) as usize];
                let tile_addr = if use_signed {
                    let base = 0x9000i32 + (tile_num as i8 as i32) * 16;
                    base as u16
                } else {
                    0x8000u16 + (tile_num as u16) * 16
                };

                let line = (sy % 8) as u16;
                let byte0 = vram[(tile_addr + line * 2 - 0x8000) as usize];
                let byte1 = vram[(tile_addr + line * 2 + 1 - 0x8000) as usize];
                let bit = 7 - (sx % 8);
                let color_id = ((byte1 >> bit) & 1) << 1 | ((byte0 >> bit) & 1);
                let shade = Self::map_palette(self.bgp, color_id);

                let idx = (y * SCREEN_W + x) * 4;
                self.fb[idx] = shade;
                self.fb[idx + 1] = shade;
                self.fb[idx + 2] = shade;
                self.fb[idx + 3] = 0xFF;
            }
        }

        // Window overlay (no sprites yet)
        if self.lcdc & 0x20 != 0 {
            let win_tile_map_base = if self.lcdc & 0x40 != 0 { 0x9C00 } else { 0x9800 };
            let win_x = self.wx.wrapping_sub(7); // WX is offset by 7
            let win_y = self.wy;
            for y in 0..SCREEN_H {
                if y < win_y as usize {
                    continue;
                }
                let wy = y - win_y as usize;
                let tile_y = wy / 8;
                for x in 0..SCREEN_W {
                    if x < win_x as usize {
                        continue;
                    }
                    let wx = x - win_x as usize;
                    let tile_x = wx / 8;
                    let tile_map_index = tile_y * 32 + tile_x;
                    let tile_num = vram[(win_tile_map_base - 0x8000 + tile_map_index) as usize];
                    let tile_addr = if use_signed {
                        let base = 0x9000i32 + (tile_num as i8 as i32) * 16;
                        base as u16
                    } else {
                        0x8000u16 + (tile_num as u16) * 16
                    };

                    let line = (wy % 8) as u16;
                    let byte0 = vram[(tile_addr + line * 2 - 0x8000) as usize];
                    let byte1 = vram[(tile_addr + line * 2 + 1 - 0x8000) as usize];
                    let bit = 7 - (wx % 8);
                    let color_id = ((byte1 >> bit) & 1) << 1 | ((byte0 >> bit) & 1);
                    let shade = Self::map_palette(self.bgp, color_id);

                    let idx = (y * SCREEN_W + x) * 4;
                    self.fb[idx] = shade;
                    self.fb[idx + 1] = shade;
                    self.fb[idx + 2] = shade;
                    self.fb[idx + 3] = 0xFF;
                }
            }
        }

        self.mode = 0; // HBlank
    }

    fn map_palette(palette: u8, color_id: u8) -> u8 {
        let shift = color_id * 2;
        match (palette >> shift) & 0x03 {
            0 => 0xFF,
            1 => 0xAA,
            2 => 0x55,
            _ => 0x00,
        }
    }

    pub fn framebuffer_rgba(&self) -> &[u8] {
        &self.fb
    }

    fn update_stat(&mut self, vblank: bool) -> bool {
        let mut irq = false;
        let lyc_match = self.ly == self.lyc;
        if lyc_match {
            self.stat |= 0x04;
        } else {
            self.stat &= !0x04;
        }

        // STAT mode bits
        self.stat = (self.stat & !0x03) | (self.mode & 0x03);

        // Interrupt conditions: bit6 (LYC), bit5 (OAM), bit4 (VBlank), bit3 (HBlank)
        if lyc_match && (self.stat & 0x40 != 0) {
            irq = true;
        }
        if self.mode == 2 && (self.stat & 0x20 != 0) {
            irq = true;
        }
        if self.mode == 1 && (self.stat & 0x10 != 0) {
            irq = true;
        }
        if self.mode == 0 && (self.stat & 0x08 != 0) {
            irq = true;
        }

        if vblank {
            self.stat |= 0x01;
        }

        irq
    }
}
