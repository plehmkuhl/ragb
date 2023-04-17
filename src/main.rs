#![feature(bigint_helper_methods)]

extern crate sdl2;

mod gba;
mod arm;
mod bus;
mod decode_arm;
mod decode_thumb;
mod alu;

use crate::gba::*;

use std::fs;
use std::mem;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::TextureQuery;

fn main() {
    // Init SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("Ragb", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    // Init emulator
    let mut gba = GbaSystem::new();

    // Print table
    for n in &gba.instruction_table {
        println!("{:032b}", n.0);
    }

    for n in &gba.thumb_instruction_table {
        println!("{:016b}", n.0);
    }

    // Load BIOS
    let bios = fs::read("gba_bios.bin").unwrap();
    for n in 0..(bios.len() / mem::size_of::<u32>()){
        let adr = n*mem::size_of::<u32>();
        gba.bios[n] = u32::from_ne_bytes(bios[adr..(adr+4)].try_into().unwrap()).to_le();
    }

    // Perform a system reset
    gba.reset();

    let ttf_context = sdl2::ttf::init().unwrap();
    let texture_creator = canvas.texture_creator();

    let mut font = ttf_context.load_font("UbuntuMono-Regular.ttf", 12).unwrap();

    let mut accumulator: u32 = 0;

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
        accumulator += CPU_TICKS_PER_SECOND;

        while accumulator > 0 {
            accumulator -= gba.emulate();
        }

        // Update debug screen
        let surface = font
            .render(format!(
                "r0:  {:#010x}\nr1:  {:#010x}\nr2:  {:#010x}\nr3:  {:#010x}\nr4:  {:#010x}\n\
                r5:  {:#010x}\n\r6:  {:#010x}\nr7:  {:#010x}\nr8:  {:#010x}\nr9:  {:#010x}\n\
                r10: {:#010x}\nr11: {:#010x}\nr12: {:#010x}\nr13: {:#010x}\nr14: {:#010x}\n\
                r15: {:#010x}",
                gba.r[0], gba.r[1], gba.r[2], gba.r[3], gba.r[4], 
                gba.r[5], gba.r[6], gba.r[7], gba.r[8], gba.r[9], 
                gba.r[10], gba.r[11], gba.r[12], gba.r[13], gba.r[14],
                gba.r[15]).as_str(),)
            .blended_wrapped(Color::RGBA(255, 255, 255, 255), canvas.logical_size().0)
            .unwrap();

        let texture = texture_creator
                .create_texture_from_surface(&surface).unwrap();

        let TextureQuery { width, height, .. } = texture.query();

        // Render screen
        canvas.clear();
        canvas.copy(&texture, None, sdl2::rect::Rect::new(0, 0, width, height));
        canvas.present();
    }
}
