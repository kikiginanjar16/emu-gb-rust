use anyhow::Result;
use clap::Parser;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod emu;

#[derive(Parser, Debug)]
struct Args {
    /// Path to .gb ROM file (use your own/homebrew ROM)
    rom: String,

    /// Scale factor for the 160x144 screen
    #[arg(long, default_value_t = 4)]
    scale: u32,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    let mut emu = emu::Emulator::new(&args.rom)?;

    let event_loop = EventLoop::new();
    let width = 160u32;
    let height = 144u32;

    let window = WindowBuilder::new()
        .with_title("GB Emulator (Rust) - Starter")
        .with_inner_size(LogicalSize::new(
            (width * args.scale) as f64,
            (height * args.scale) as f64,
        ))
        .with_min_inner_size(LogicalSize::new(
            (width * args.scale) as f64,
            (height * args.scale) as f64,
        ))
        .build(&event_loop)?;

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(width, height, surface_texture)?;

    // Simple input state (Game Boy buttons)
    let mut input = emu::JoypadState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::Resized(size) => {
                    pixels.resize_surface(size.width, size.height).ok();
                    window.request_redraw();
                }

                WindowEvent::KeyboardInput { input: key, .. } => {
                    handle_key(&mut input, key);
                    emu.set_joypad(input);
                }

                _ => {}
            },

            Event::RedrawRequested(_) => {
                // Run enough cycles for one frame (approx 70224 cycles/frame on DMG)
                emu.run_frame();

                let frame = pixels.frame_mut();
                frame.copy_from_slice(emu.framebuffer_rgba());

                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                }
            }

            Event::MainEventsCleared => {
                window.request_redraw();
            }

            _ => {}
        }
    });

    // unreachable
}

fn handle_key(state: &mut emu::JoypadState, key: KeyboardInput) {
    let pressed = key.state == ElementState::Pressed;

    if let Some(k) = key.virtual_keycode {
        match k {
            // D-Pad
            VirtualKeyCode::Up => state.up = pressed,
            VirtualKeyCode::Down => state.down = pressed,
            VirtualKeyCode::Left => state.left = pressed,
            VirtualKeyCode::Right => state.right = pressed,

            // Buttons
            VirtualKeyCode::Z => state.a = pressed,      // A
            VirtualKeyCode::X => state.b = pressed,      // B
            VirtualKeyCode::Return => state.start = pressed,
            VirtualKeyCode::RShift => state.select = pressed,

            _ => {}
        }
    }
}
