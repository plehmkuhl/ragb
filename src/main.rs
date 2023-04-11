extern crate sdl2;

mod emulator;
mod bus;

use crate::emulator::*;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

fn main() {
    // Init SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Ragb", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();

    // Init emulator
    let mut emulator = EmulatorState::new();

    emulator.read_bus(0x0);
    emulator.write_bus(0x0, bus::BusValue::Dword(0));

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        // Process events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown {keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }

        // Update emulation state

        // Render screen
        canvas.clear();
        canvas.present();
    }
}
