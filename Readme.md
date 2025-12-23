# Game Boy Emulator (Rust)

Starter Game Boy emulator built with `winit` + `pixels`. CPU/PPU are highly incomplete (CPU only a few opcodes, PPU renders a checkerboard so you know it runs). Expect crashes if the ROM touches unimplemented pieces.

## Requirements
- Rust toolchain (stable) and Cargo
- A Game Boy `.gb` ROM you own/homebrew

## Run
```bash
cargo run --release -- /path/to/rom.gb --scale 4
```

## Controls
- D-Pad: Arrow keys
- A / B: Z / X
- Start / Select: Enter / Right Shift

## Status / Next
- Missing most CPU opcodes, timers/APU, interrupts, and PPU rendering.
- Cartridge handling is ROM-only (no MBC). VRAM/OAM/IO largely stubbed.
- Good starting point to flesh out opcodes, implement PPU modes, and hook up timers + interrupts.
