#![feature(bigint_helper_methods)]

extern crate sdl2;
extern crate gl;

mod gba;
mod arm;
mod bus;
mod decode_arm;
mod decode_thumb;
mod alu;
mod gdb;
mod io_register;

use crate::arm::ProgramStatus;
use crate::gba::*;
use crate::gdb::{ GbaDebugCommand, GbaDebugCommandResult };

use gl::types::{GLenum, GLuint, GLint, GLchar, GLfloat, GLsizeiptr, GLboolean};
use tokio::sync::watch;

use std::ffi::CString;
use std::{fs, ptr};
use std::mem;
use std::rc::Rc;
use std::thread;
use std::sync::mpsc;
use std::net::{TcpListener, TcpStream};
use std::io;
use std::sync::Arc;

use gdbstub::stub::{GdbStubBuilder, SingleThreadStopReason};
use gdbstub::conn::ConnectionExt;
use gdbstub::common::Signal;

use gdbstub_arch::arm::reg::ArmCoreRegs;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::TextureQuery;

fn wait_for_gdb_connection(port: u16) -> io::Result<TcpStream> {
    let sockaddr = format!("localhost:{}", port);
    eprintln!("Waiting for a GDB connection on {:?}...", sockaddr);
    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;

    // Blocks until a GDB client connects via TCP.
    // i.e: Running `target remote localhost:<port>` from the GDB prompt.

    eprintln!("Debugger connected from {}", addr);
    Ok(stream) // `TcpStream` implements `gdbstub::Connection`
}

fn compile_shader(src: &[u8], ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);

        // Attempt to compile the shader
        let c_str = CString::new(src).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Failt on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::new();
            buf.set_len((len as usize) - 1);
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            panic!("{}", String::from_utf8(buf).ok().expect("ShaderInfoLog not valid utf8"));
        }
    }

    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::new();
            buf.set_len((len as usize) - 1);
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            panic!("{}", String::from_utf8(buf).ok().expect("ProgramInfoLog not valid utf8"));
        }

        program
    }
}

fn main() {
    // Init SDL
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    video_subsystem.gl_attr().set_context_profile(sdl2::video::GLProfile::Core);
    video_subsystem.gl_attr().set_context_version(3, 2);

    // Create GL window and context
    let window = video_subsystem.window("Ragb", 800, 600)
        .opengl()
        .resizable()
        .build()
        .unwrap();

    let context = window.gl_create_context().unwrap();
    window.gl_make_current(&context).unwrap();

    gl::load_with(|s| video_subsystem.gl_get_proc_address(s) as *const std::os::raw::c_void);
    //video_subsystem.gl_set_swap_interval(1).unwrap();

    // Compile shader
    let vs = compile_shader(fs::read("shader/vertex_shader.glsl").unwrap().as_slice(), gl::VERTEX_SHADER);
    let fs = compile_shader(fs::read("shader/fragment_shader.glsl").unwrap().as_slice(), gl::FRAGMENT_SHADER);

    let program = link_program(vs, fs);

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

    let (ui_tx, ui_rx) = mpsc::channel::<()>();
    let (gdb_tx, gdb_rx) = mpsc::channel::<gdb::GbaDebugCommand>();
    let (signal_tx, signal_rx) = mpsc::channel::<Signal>();
    let (state_tx, state_rx) = watch::channel::<Option<SingleThreadStopReason<u32>>>(None);

    // Immediately send the SIGINT signal to the emulation thread
    let _ = signal_tx.send(Signal::SIGINT);

    // Start debug thread
    thread::spawn(move || {
        // Setup debugger
        let target = gdb::GbaTarget { emulation_tx_channel: gdb_tx, signal_tx_channel: signal_tx, state_rx_channel: state_rx };

        // Wait for connection
        if let Ok(con) = wait_for_gdb_connection(2345) {
            if let Ok(debugger) = GdbStubBuilder::<gdb::GbaTarget, Box<dyn ConnectionExt<Error = std::io::Error>>>::new(Box::new(con)).build() {
                gdb::gdb_event_loop_thread(debugger, target);
            }
        }
    });


    // Start emulation thread
    thread::spawn(move || {
        enum EmulationState {
            Continue,
            Halt,
        }

        let mut state = EmulationState::Continue;
        let mut accumulator: u32 = 0;

        'main_loop: loop {
            match state {
                EmulationState::Continue => {
                    let _ = state_tx.send(None);

                    'emulation: loop {
                        if let Ok(signal) = signal_rx.try_recv() {
                            match signal {
                                Signal::SIGINT => {
                                    state = EmulationState::Halt;
                                    let _ = state_tx.send(Some(SingleThreadStopReason::Signal(signal)));

                                    break 'emulation;
                                },
                                _ => {
                                    println!("Unhandled signal: {}", signal);
                                }
                            }
                        }
            
                        // Update emulation state
                        while accumulator > 0 {
                            match gba.emulate() {
                                EmulationResult::Cycles(c) => accumulator -= c,
                                EmulationResult::Exception => {
                                    state = EmulationState::Halt;
                                    let _ = state_tx.send(Some(SingleThreadStopReason::Signal(Signal::EXC_BAD_ACCESS)));
                                    break 'emulation;
                                }
                            }

                            // Check breakpoints
                            for bp in gba.breakpoints.as_slice() {
                                if bp.0 == gba.pc {
                                    state = EmulationState::Halt;
                                    let _ = state_tx.send(Some(SingleThreadStopReason::SwBreak(())));
                                    break 'emulation;
                                }
                            }
                        }
            
                        let _ = ui_tx.send(());
                        accumulator += CPU_TICKS_PER_SECOND;
                    }
                },
                EmulationState::Halt => {
                    // If no other reason was given, give swbreak
                    if let None = *state_tx.borrow() {
                        let _ = state_tx.send(Some(SingleThreadStopReason::SwBreak(())));
                    }

                    if let Ok(cmd) = gdb_rx.recv() {
                        match cmd {
                            GbaDebugCommand::ReadRegisters { tx } => {
                                let _ = tx.send(GbaDebugCommandResult::Registers { 
                                    regs:  ArmCoreRegs {
                                        r: [
                                            gba.read_register(Register::R0),
                                            gba.read_register(Register::R1),
                                            gba.read_register(Register::R2),
                                            gba.read_register(Register::R3),
                                            gba.read_register(Register::R4),
                                            gba.read_register(Register::R5),
                                            gba.read_register(Register::R6),
                                            gba.read_register(Register::R7),
                                            gba.read_register(Register::R8),
                                            gba.read_register(Register::R9),
                                            gba.read_register(Register::R10),
                                            gba.read_register(Register::R11),
                                            gba.read_register(Register::R12),
                                        ],
                                        sp: gba.read_register(Register::R13),
                                        lr: gba.read_register(Register::R14),
                                        pc: gba.pc,
                                        cpsr: gba.cpsr.bits(),
                                    }
                                });
                            },
                            GbaDebugCommand::WriteRegisters { tx, regs } => {
                                gba.write_register(Register::R0, regs.r[0]);
                                gba.write_register(Register::R1, regs.r[1]);
                                gba.write_register(Register::R2, regs.r[2]);
                                gba.write_register(Register::R3, regs.r[3]);
                                gba.write_register(Register::R4, regs.r[4]);
                                gba.write_register(Register::R5, regs.r[5]);
                                gba.write_register(Register::R6, regs.r[6]);
                                gba.write_register(Register::R7, regs.r[7]);
                                gba.write_register(Register::R8, regs.r[8]);
                                gba.write_register(Register::R9, regs.r[9]);
                                gba.write_register(Register::R10, regs.r[10]);
                                gba.write_register(Register::R11, regs.r[11]);
                                gba.write_register(Register::R12, regs.r[12]);
                                gba.write_register(Register::R13, regs.sp);
                                gba.write_register(Register::R14, regs.lr);
                                gba.write_register(Register::R15, regs.pc);
                                gba.cpsr = ProgramStatus::from_bits_truncate(regs.cpsr);

                                let _ = tx.send(GbaDebugCommandResult::Executed);
                            },
                            GbaDebugCommand::ReadAddress { tx, start_address, len } => {
                                //println!("Read address {:08x} {}", start_address, len);

                                match {
                                    let mut data = Vec::<u8>::new();
                                    for adr in start_address..start_address.saturating_add(len as u32) {
                                        if let Some(b) = gba.read_bus_byte(adr) {
                                            data.push(b);
                                        } else {
                                            data.push(0);
                                            //return Err::<Vec<u8>, ()>(());
                                        }
                                    }

                                    // Fill up using zeros
                                    data.resize(len, 0);

                                    Ok::<Vec<u8>, ()>(data)
                                } {
                                    Ok(data) => {
                                        let _ = tx.send(GbaDebugCommandResult::Data { bytes: data });
                                    },
                                    Err(e) => {
                                        let _ = tx.send(GbaDebugCommandResult::Executed);
                                    }
                                }
                            },
                            GbaDebugCommand::WriteAddress { tx, start_address, data } => {
                                match {
                                    let mut adr: u32 = start_address;
                                    for b in data {
                                        gba.write_bus(adr, bus::BusValue::Byte(b));
                                    }

                                    Ok::<(), ()>(())
                                } {
                                    Ok(()) => {
                                        let _ = tx.send(GbaDebugCommandResult::Executed);
                                    },
                                    Err(e) => {
                                        let _ = tx.send(GbaDebugCommandResult::Executed);
                                    }
                                }
                            },
                            GbaDebugCommand::Resume { tx } => {
                                state = EmulationState::Continue;
                                let _ = state_tx.send(None);
                                let _ = tx.send(GbaDebugCommandResult::Executed);
                            },
                            GbaDebugCommand::SingleStep { tx } => {
                                gba.emulate();
                                let _ = state_tx.send(Some(SingleThreadStopReason::DoneStep));
                                let _ = tx.send(GbaDebugCommandResult::Executed);
                            },
                            GbaDebugCommand::AddSwBreakpoint { tx, addr, kind } => {
                                gba.breakpoints.push((addr, kind));
                                let _ = tx.send(GbaDebugCommandResult::Executed);
                            },
                            GbaDebugCommand::RemoveSwBreakpoint { tx, addr, kind } => {
                                gba.breakpoints.push((addr, kind));
                                let _ = tx.send(GbaDebugCommandResult::Executed);

                            }
                        }
                    } else {
                        panic!("Failed to receive debug command");
                    }
                }
            }
        }
    });

    let mut vao = 0;
    let mut vbo = 0;

    static VERTEX_DATA: [GLfloat; 8] = [
        0.0,  0.0,
        0.0,  1.0,
        1.0,  1.0,
        1.0,  0.0];

    unsafe {
        // Create Vertex Array Object
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
            (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            mem::transmute(&VERTEX_DATA[0]),
            gl::STATIC_DRAW);

        // Use shader program
        gl::UseProgram(program);
        gl::BindFragDataLocation(program, 0,
            CString::new("out_color").unwrap().as_ptr());
        
        // Specify the layout of the vertex data
        let pos_attr = gl::GetAttribLocation(program, CString::new("position").unwrap().as_ptr());
        gl::EnableVertexAttribArray(pos_attr as GLuint);
        gl::VertexAttribPointer(pos_attr as GLuint, 2, gl::FLOAT, gl::FALSE as GLboolean, 0, ptr::null());
    }

    unsafe {
        gl::Viewport(0, 0, 240+68, 160+68);
        gl::ClearColor(0.3, 0.3, 0.3, 1.0);
    }

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        // Process events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown {keycode: Some(Keycode::Escape), .. } => break 'running,
                _ => {}
            }
        } 

        let _ = ui_rx.recv();

        // Render screen
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::DrawArrays(gl::QUADS, 0, 4);
        }

        window.gl_swap_window();
        //frame_count = frame_count.wrapping_add(1);
        //println!("Frame: {}", frame_count);
    }
}
