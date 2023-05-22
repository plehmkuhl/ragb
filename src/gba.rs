use std::{collections::VecDeque, fs, io, string::String, os::windows::process};

use bitflags::BitFlags;
use nom::{multi, number, Finish};

use crate::{
    arm::*, 
    bus::BusValue, 
    decode_arm::{InstructionTableEntry, generate_arm_instruction_table, ArmInstruction},
    decode_thumb::{ThumbInstructionTableEntry, generate_thumb_instruction_table},
    io_register::{self, IORegister},
};

use std::fmt;
use std::sync::mpsc::Sender;

pub const CPU_TICKS_PER_SECOND: u32 = 16780000; //280896 ; //16780000;

#[derive(PartialEq)]
#[derive(Copy, Clone)]
pub enum Register {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl TryFrom<u8> for Register {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Register::R0),
            1 => Ok(Register::R1),
            2 => Ok(Register::R2),
            3 => Ok(Register::R3),
            4 => Ok(Register::R4),
            5 => Ok(Register::R5),
            6 => Ok(Register::R6),
            7 => Ok(Register::R7),
            8 => Ok(Register::R8),
            9 => Ok(Register::R9),
            10 => Ok(Register::R10),
            11 => Ok(Register::R11),
            12 => Ok(Register::R12),
            13 => Ok(Register::R13),
            14 => Ok(Register::R14),
            15 => Ok(Register::R15),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Register::R0 => write!(f, "r0"),
            Register::R1 => write!(f, "r1"),
            Register::R2 => write!(f, "r2"),
            Register::R3 => write!(f, "r3"),
            Register::R4 => write!(f, "r4"),
            Register::R5 => write!(f, "r5"),
            Register::R6 => write!(f, "r6"),
            Register::R7 => write!(f, "r7"),
            Register::R8 => write!(f, "r8"),
            Register::R9 => write!(f, "r9"),
            Register::R10 => write!(f, "r10"),
            Register::R11 => write!(f, "r11"),
            Register::R12 => write!(f, "r12"),
            Register::R13 => write!(f, "sp"),
            Register::R14 => write!(f, "lr"),
            Register::R15 => write!(f, "pc"),
        }
    }
}

pub enum EmulationResult {
    Cycles(u32),
    Exception,
}

#[derive(PartialEq)]
pub enum ExceptionType {
    Reset,
    UndefinedInstruction,
    SoftwareInterrupt,
    PrefetchAbort,
    DataAbort,
    IRQ,
    FIQ,
}

pub enum VideoEvent {
    VRamUpdate { start_address: u32, data: Vec<u16> },
    PRamUpdate { start_address: u32, data: Vec<u16> },
    ORamUpdate { start_address: u32, data: Vec<u32> },
    FrameUpdate,
}

pub struct GbaSystem {
    // Memory (32-Bit bus)
    pub bios:Vec<u32>, 
    pub iwram:Vec<u32>,
    pub oam:Vec<u32>,
    //pub io:Vec<u32>,

    // Memory (16-Bit bus)
    pub ewram:Vec<u16>,
    pub vram:Vec<u16>,
    pub pram:Vec<u16>,
    pub pack:Vec<u16>,

    // Memory (8-Bit bus)
    pub sram:Vec<u8>,

    // Hardware
    pub io_register: IORegister,

    // Registers
    pub r:[u32; 16],
    pub r_fiq:[u32; 7],
    pub r_svc:[u32; 2],
    pub r_abt:[u32; 2],
    pub r_irq:[u32; 2],
    pub r_und:[u32; 2],

    pub cpsr: ProgramStatus,
    pub spsr_fiq: ProgramStatus,
    pub spsr_svc: ProgramStatus,
    pub spsr_abt: ProgramStatus,
    pub spsr_irq: ProgramStatus,
    pub spsr_und: ProgramStatus,

    // Normally r15 is considered the pc, but for debugging purposes we need
    // a pc pointing to the currently executing instruction
    pub pc: u32,
    pub pc_dirty: bool,

    // CPU
    pub instruction_table: Vec<InstructionTableEntry>,
    pub thumb_instruction_table: Vec<ThumbInstructionTableEntry>,

    pub instruction_cache: VecDeque<(u32, Option<ArmInstruction>)>,

    // Debug
    pub breakpoints: Vec<(u32, gdbstub_arch::arm::ArmBreakpointKind)>,
    pub decompile: Vec<String>,

    // LCD
    pub lcd_accumulator: u32,
    pub lcd_cur_v: u16,
    pub lcd_cur_h: u16,

    // Synchronization
    pub video_events: Sender<VideoEvent>,
}

impl GbaSystem {
    pub fn new
    (frame_sender: Sender<VideoEvent>) -> GbaSystem {
        GbaSystem {
            bios: vec![0; 4096],
            iwram: vec![0; 8192],
            oam: vec![0; 256],
            ewram: vec![0; 131072],
            vram: vec![0; 49152],
            pram: vec![0; 512],
            pack: vec![0; 16777216],
            sram: vec![0; 65536],
            io_register: IORegister::new(),
            r: [0; 16],
            r_fiq: [0; 7],
            r_svc: [0; 2],
            r_abt: [0; 2],
            r_irq: [0; 2],
            r_und: [0; 2],
            cpsr: ProgramStatus::empty(),
            spsr_fiq: ProgramStatus::empty(),
            spsr_svc: ProgramStatus::empty(),
            spsr_abt: ProgramStatus::empty(),
            spsr_irq: ProgramStatus::empty(),
            spsr_und: ProgramStatus::empty(),
            pc: 0,
            pc_dirty: false,
            instruction_table: generate_arm_instruction_table(),
            thumb_instruction_table: generate_thumb_instruction_table(),
            instruction_cache: VecDeque::new(),
            breakpoints: Vec::new(),
            decompile: vec!["".into(); 0x3FFF],
            lcd_accumulator: 0,
            lcd_cur_h: 0,
            lcd_cur_v: 0,
            video_events: frame_sender,
        }
    }

    pub fn read_register(&self, r: Register) -> u32 {
        match self.cpsr.get_mode() {
            Mode::User |
            Mode::System => match r {
                Register::R0 => self.r[0],
                Register::R1 => self.r[1],
                Register::R2 => self.r[2],
                Register::R3 => self.r[3],
                Register::R4 => self.r[4],
                Register::R5 => self.r[5],
                Register::R6 => self.r[6],
                Register::R7 => self.r[7],
                Register::R8 => self.r[8],
                Register::R9 => self.r[9],
                Register::R10 => self.r[10],
                Register::R11 => self.r[11],
                Register::R12 => self.r[12],
                Register::R13 => self.r[13],
                Register::R14 => self.r[14],
                Register::R15 => self.r[15],
            },
            Mode::FIQ => match r {
                Register::R0 => self.r[0],
                Register::R1 => self.r[1],
                Register::R2 => self.r[2],
                Register::R3 => self.r[3],
                Register::R4 => self.r[4],
                Register::R5 => self.r[5],
                Register::R6 => self.r[6],
                Register::R7 => self.r[7],
                Register::R8 => self.r_fiq[0],
                Register::R9 => self.r_fiq[1],
                Register::R10 => self.r_fiq[2],
                Register::R11 => self.r_fiq[3],
                Register::R12 => self.r_fiq[4],
                Register::R13 => self.r_fiq[5],
                Register::R14 => self.r_fiq[6],
                Register::R15 => self.r[15],
            },
            Mode::IRQ => match r {
                Register::R0 => self.r[0],
                Register::R1 => self.r[1],
                Register::R2 => self.r[2],
                Register::R3 => self.r[3],
                Register::R4 => self.r[4],
                Register::R5 => self.r[5],
                Register::R6 => self.r[6],
                Register::R7 => self.r[7],
                Register::R8 => self.r[8],
                Register::R9 => self.r[9],
                Register::R10 => self.r[10],
                Register::R11 => self.r[11],
                Register::R12 => self.r[12],
                Register::R13 => self.r_irq[0],
                Register::R14 => self.r_irq[1],
                Register::R15 => self.r[15],
            },
            Mode::Supervisor => match r {
                Register::R0 => self.r[0],
                Register::R1 => self.r[1],
                Register::R2 => self.r[2],
                Register::R3 => self.r[3],
                Register::R4 => self.r[4],
                Register::R5 => self.r[5],
                Register::R6 => self.r[6],
                Register::R7 => self.r[7],
                Register::R8 => self.r[8],
                Register::R9 => self.r[9],
                Register::R10 => self.r[10],
                Register::R11 => self.r[11],
                Register::R12 => self.r[12],
                Register::R13 => self.r_svc[0],
                Register::R14 => self.r_svc[1],
                Register::R15 => self.r[15],
            },
            Mode::Abort => match r {
                Register::R0 => self.r[0],
                Register::R1 => self.r[1],
                Register::R2 => self.r[2],
                Register::R3 => self.r[3],
                Register::R4 => self.r[4],
                Register::R5 => self.r[5],
                Register::R6 => self.r[6],
                Register::R7 => self.r[7],
                Register::R8 => self.r[8],
                Register::R9 => self.r[9],
                Register::R10 => self.r[10],
                Register::R11 => self.r[11],
                Register::R12 => self.r[12],
                Register::R13 => self.r_abt[0],
                Register::R14 => self.r_abt[1],
                Register::R15 => self.r[15],
            },
            Mode::Undefined => match r {
                Register::R0 => self.r[0],
                Register::R1 => self.r[1],
                Register::R2 => self.r[2],
                Register::R3 => self.r[3],
                Register::R4 => self.r[4],
                Register::R5 => self.r[5],
                Register::R6 => self.r[6],
                Register::R7 => self.r[7],
                Register::R8 => self.r[8],
                Register::R9 => self.r[9],
                Register::R10 => self.r[10],
                Register::R11 => self.r[11],
                Register::R12 => self.r[12],
                Register::R13 => self.r_und[0],
                Register::R14 => self.r_und[1],
                Register::R15 => self.r[15],
            },
        }
    }

    pub fn write_register(&mut self, r: Register, v: u32) {
        match self.cpsr.get_mode() {
            Mode::User |
            Mode::System => match r {
                Register::R0 => self.r[0] = v,
                Register::R1 => self.r[1] = v,
                Register::R2 => self.r[2] = v,
                Register::R3 => self.r[3] = v,
                Register::R4 => self.r[4] = v,
                Register::R5 => self.r[5] = v,
                Register::R6 => self.r[6] = v,
                Register::R7 => self.r[7] = v,
                Register::R8 => self.r[8] = v,
                Register::R9 => self.r[9] = v,
                Register::R10 => self.r[10] = v,
                Register::R11 => self.r[11] = v,
                Register::R12 => self.r[12] = v,
                Register::R13 => self.r[13] = v,
                Register::R14 => self.r[14] = v,
                Register::R15 => { self.pc_dirty = true; self.r[15] = v },
            },
            Mode::FIQ => match r {
                Register::R0 => self.r[0] = v,
                Register::R1 => self.r[1] = v,
                Register::R2 => self.r[2] = v,
                Register::R3 => self.r[3] = v,
                Register::R4 => self.r[4] = v,
                Register::R5 => self.r[5] = v,
                Register::R6 => self.r[6] = v,
                Register::R7 => self.r[7] = v,
                Register::R8 => self.r_fiq[0] = v,
                Register::R9 => self.r_fiq[1] = v,
                Register::R10 => self.r_fiq[2] = v,
                Register::R11 => self.r_fiq[3] = v,
                Register::R12 => self.r_fiq[4] = v,
                Register::R13 => self.r_fiq[5] = v,
                Register::R14 => self.r_fiq[6] = v,
                Register::R15 => { self.pc_dirty = true; self.r[15] = v },
            },
            Mode::IRQ => match r {
                Register::R0 => self.r[0] = v,
                Register::R1 => self.r[1] = v,
                Register::R2 => self.r[2] = v,
                Register::R3 => self.r[3] = v,
                Register::R4 => self.r[4] = v,
                Register::R5 => self.r[5] = v,
                Register::R6 => self.r[6] = v,
                Register::R7 => self.r[7] = v,
                Register::R8 => self.r[8] = v,
                Register::R9 => self.r[9] = v,
                Register::R10 => self.r[10] = v,
                Register::R11 => self.r[11] = v,
                Register::R12 => self.r[12] = v,
                Register::R13 => self.r_irq[0] = v,
                Register::R14 => self.r_irq[1] = v,
                Register::R15 => { self.pc_dirty = true; self.r[15] = v },
            },
            Mode::Supervisor => match r {
                Register::R0 => self.r[0] = v,
                Register::R1 => self.r[1] = v,
                Register::R2 => self.r[2] = v,
                Register::R3 => self.r[3] = v,
                Register::R4 => self.r[4] = v,
                Register::R5 => self.r[5] = v,
                Register::R6 => self.r[6] = v,
                Register::R7 => self.r[7] = v,
                Register::R8 => self.r[8] = v,
                Register::R9 => self.r[9] = v,
                Register::R10 => self.r[10] = v,
                Register::R11 => self.r[11] = v,
                Register::R12 => self.r[12] = v,
                Register::R13 => self.r_svc[0] = v,
                Register::R14 => self.r_svc[1] = v,
                Register::R15 => { self.pc_dirty = true; self.r[15] = v },
            },
            Mode::Abort => match r {
                Register::R0 => self.r[0] = v,
                Register::R1 => self.r[1] = v,
                Register::R2 => self.r[2] = v,
                Register::R3 => self.r[3] = v,
                Register::R4 => self.r[4] = v,
                Register::R5 => self.r[5] = v,
                Register::R6 => self.r[6] = v,
                Register::R7 => self.r[7] = v,
                Register::R8 => self.r[8] = v,
                Register::R9 => self.r[9] = v,
                Register::R10 => self.r[10] = v,
                Register::R11 => self.r[11] = v,
                Register::R12 => self.r[12] = v,
                Register::R13 => self.r_abt[0] = v,
                Register::R14 => self.r_abt[1] = v,
                Register::R15 => { self.pc_dirty = true; self.r[15] = v },
            },
            Mode::Undefined => match r {
                Register::R0 => self.r[0] = v,
                Register::R1 => self.r[1] = v,
                Register::R2 => self.r[2] = v,
                Register::R3 => self.r[3] = v,
                Register::R4 => self.r[4] = v,
                Register::R5 => self.r[5] = v,
                Register::R6 => self.r[6] = v,
                Register::R7 => self.r[7] = v,
                Register::R8 => self.r[8] = v,
                Register::R9 => self.r[9] = v,
                Register::R10 => self.r[10] = v,
                Register::R11 => self.r[11] = v,
                Register::R12 => self.r[12] = v,
                Register::R13 => self.r_und[0] = v,
                Register::R14 => self.r_und[1] = v,
                Register::R15 => { self.pc_dirty = true; self.r[15] = v },
            },
        }
    }

    pub fn reset(&mut self) {
        self.raise_exception(ExceptionType::Reset);
    }

    pub fn raise_exception(&mut self, exception_type: ExceptionType) {
        // Save current state
        let return_link = self.pc;
        let spsr = self.cpsr;

        // Switch mode
        self.cpsr.set_mode(match exception_type {
            ExceptionType::Reset |
            ExceptionType::SoftwareInterrupt => Mode::Supervisor,
            ExceptionType::UndefinedInstruction => Mode::Undefined,
            ExceptionType::PrefetchAbort => Mode::Abort,
            ExceptionType::DataAbort => Mode::Abort,
            ExceptionType::IRQ => Mode::IRQ,
            ExceptionType::FIQ => Mode::FIQ,
        });

        // Update mode registers
        let _ = self.set_spsr(spsr);
        self.write_register(Register::R14, return_link);

        // Switch to ARM mode
        self.cpsr.set(ProgramStatus::FLAG_T, false);

        if exception_type == ExceptionType::Reset || exception_type == ExceptionType::FIQ {
            self.cpsr.set(ProgramStatus::FLAG_F, true);
        }

        self.cpsr.set(ProgramStatus::FLAG_I, true);

        if exception_type != ExceptionType::UndefinedInstruction || exception_type == ExceptionType::SoftwareInterrupt {
            self.cpsr.set(ProgramStatus::FLAG_A, true);
        }

        self.write_register(Register::R15, match exception_type {
            ExceptionType::Reset => 0x00000000,
            ExceptionType::UndefinedInstruction => 0x00000004,
            ExceptionType::SoftwareInterrupt => 0x00000008,
            ExceptionType::PrefetchAbort => 0x0000000C,
            ExceptionType::DataAbort => 0x00000010,
            ExceptionType::IRQ => 0x00000018,
            ExceptionType::FIQ => 0x0000001C,
        });
    }

    fn emulate_cpu(&mut self) -> EmulationResult {
        let instruction_size = self.instruction_size();

        // Clear instruction cache if pc changed since last fetch cycle
        if self.pc_dirty {
            self.instruction_cache.clear();
            self.pc_dirty = false;
        }

        /*if self.r[15] > 0x3FFF {
            println!("Leaving rom!");
            return EmulationResult::Exception
        }*/

        // Fill instruction cache
        while self.instruction_cache.len() < 2 {
            let mut two_part = false;
            let adr = self.r[15];
            let inst = 
                // Thumb mode
                if self.cpsr.contains(ProgramStatus::FLAG_T) {
                    match self.read_bus_half_word(adr) {
                        Some(inst) => {
                            self.decode_thumb_instruction(inst)
                        },
                        _ => None,
                    }
                // Arm Mode
                } else {
                    match self.read_bus_word(adr) {
                        Some(inst) => self.decode_instruction(inst),
                        _ => None,
                    }
                };

            self.instruction_cache.push_back((adr, inst));
            self.r[15] = self.r[15].wrapping_add(instruction_size);

            if two_part { break; }
        }

        //self.last_pc = self.r[15];

        // Pop and execute instruction
        let decoded = self.instruction_cache.pop_front().unwrap();
        let mut ticks = 0;

        match decoded.1 {
            Some(inst) => {
                //if (self.r[15] - instruction_size * 2) < 0x120 || (self.r[15] - instruction_size * 2) > 0x124 {
                println!("{:#08x} {} {} cpsr: {}", decoded.0, if self.cpsr.contains(ProgramStatus::FLAG_T) { "T" } else { " " }, inst, self.cpsr);
                //}

                /*match inst {
                    ArmInstruction::LoadStoreMultiple { .. } => println!("{:#08x} {} {}", decoded.0, if self.cpsr.contains(ProgramStatus::FLAG_T) { "T" } else { " " }, inst),
                    _ => (),
                }*/

                //let old_cpsr = self.cpsr;

                /*println!("R0: {:08x} R1: {:08x} R2: {:08x} R3: {:08x}", self.read_register(Register::R0), self.read_register(Register::R1), self.read_register(Register::R2), self.read_register(Register::R3));
                println!("R4: {:08x} R5: {:08x} R6: {:08x} R7: {:08x}", self.read_register(Register::R4), self.read_register(Register::R5), self.read_register(Register::R6), self.read_register(Register::R7));
                println!("R8: {:08x} R9: {:08x} R10: {:08x} R11: {:08x}", self.read_register(Register::R8), self.read_register(Register::R9), self.read_register(Register::R10), self.read_register(Register::R11));
                println!("R12: {:08x} SP: {:08x} LR: {:08x} PC: {:08x}", self.read_register(Register::R12), self.read_register(Register::R13), self.read_register(Register::R14), self.read_register(Register::R15));
                println!("cpsr: {}", self.cpsr);
                println!("{:#08x} {} {}", decoded.0, if self.cpsr.contains(ProgramStatus::FLAG_T) { "T" } else { " " }, inst);*/

                ticks = self.execute(&inst);

                /*if old_cpsr.bits() != self.cpsr.bits() {
                    println!("{:#08x} {} {}", decoded.0, if self.cpsr.contains(ProgramStatus::FLAG_T) { "T" } else { " " }, inst);

                    assert!(true);
                } else {

                }*/

                /*if self.decompile[decoded.0 as usize].is_empty() {
                    self.decompile[decoded.0 as usize] = format!("{:#08x} {} {}", decoded.0, if self.cpsr.contains(ProgramStatus::FLAG_T) { "T" } else { " " }, inst);
                }*/

                //assert_ne!(self.read_register(Register::R12), 0x8c000008, "Register trap");
                //assert_ne!(self.read_register(Register::R1), 0x3000089, "Register trap");
                //assert_ne!(self.read_register(Register::R0), 0x8000005, "Register trap");

                if self.r[15] == 0xfffffffc {
                    self.pc = decoded.0;
                    return EmulationResult::Exception;
                }

                // Update PC with location of next instruction
                if self.pc_dirty {
                    self.pc = self.r[15];
                } else {
                    self.pc = self.instruction_cache.front().unwrap().0;
                }
            },
            None => {
                println!("R0: {} R1: {} R2: {} R3: {}", self.read_register(Register::R0), self.read_register(Register::R1), self.read_register(Register::R2), self.read_register(Register::R3));
                println!("R4: {} R5: {} R6: {} R7: {}", self.read_register(Register::R4), self.read_register(Register::R5), self.read_register(Register::R6), self.read_register(Register::R7));
                println!("R8: {} R9: {} R10: {} R11: {}", self.read_register(Register::R8), self.read_register(Register::R9), self.read_register(Register::R10), self.read_register(Register::R11));
                println!("R12: {} SP: {} LR: {} PC: {}", self.read_register(Register::R12), self.read_register(Register::R13), self.read_register(Register::R14), self.read_register(Register::R15));
                println!("cpsr: {}", self.cpsr);

                println!("Invalid instruction at {:08x}!", decoded.0);
                self.raise_exception(ExceptionType::UndefinedInstruction);
            },
        }

        EmulationResult::Cycles(ticks)
    }

    pub fn emulate(&mut self) -> EmulationResult {
        let r = self.emulate_cpu();

        match r {
            EmulationResult::Cycles(c) => self.lcd_accumulator += c,
            _ => (),
        }

        // Advance pixel clock
        while self.lcd_accumulator >= 4 {
            self.lcd_cur_h += 1;

            // H blank
            if self.lcd_cur_h == 240 {
                if self.io_register.signal_interrupt(io_register::Interrupt::LcdHBlank) {
                    self.raise_exception(ExceptionType::IRQ);
                }
            }

            // Scanline finished
            if self.lcd_cur_h >= 308 {
                self.lcd_cur_v += 1;
                self.lcd_cur_h = 0;
            }

            // V blank
            if self.lcd_cur_v == 160 {
                if self.io_register.signal_interrupt(io_register::Interrupt::LcdVBlank) {
                    self.raise_exception(ExceptionType::IRQ);
                }
            }

            // Screen finished
            if self.lcd_cur_v >= 228 {
                self.lcd_cur_v = 0;
                self.lcd_cur_h = 0;

                println!("Mode: {}, FS: {}, Blank: {}, BG: [{},{},{},{}]", 
                    self.io_register.disp_cnt & 0x7, 
                    self.io_register.disp_cnt & 0x10,
                    (self.io_register.disp_cnt & 0x80) != 0,
                    (self.io_register.disp_cnt & 0x100) != 0,
                    (self.io_register.disp_cnt & 0x200) != 0,
                    (self.io_register.disp_cnt & 0x400) != 0,
                    (self.io_register.disp_cnt & 0x800) != 0);

                // Send video ram updates
                let _ = self.video_events.send(VideoEvent::PRamUpdate { start_address: 0, data: self.pram.clone() });
                let _ = self.video_events.send(VideoEvent::VRamUpdate { start_address: 0, data: self.vram.clone() });
                let _ = self.video_events.send(VideoEvent::ORamUpdate { start_address: 0, data: self.oam.clone() });

                // Send frame update event
                let _ = self.video_events.send(VideoEvent::FrameUpdate);

                // Output decompile listing and quit
                let mut listing = String::new();
                let mut last_a = 0;

                for (a, l) in self.decompile.iter().enumerate() {
                    if !l.is_empty() {
                        if a.abs_diff(last_a) > 4 {
                            listing.push_str("\n");
                        }

                        last_a = a;
                        listing.push_str(format!("{}\n", l).as_str());
                    }
                }

                //fs::write("listing.txt", listing);
                //panic!("Got to first frame!");
            }

            self.lcd_accumulator -= 4;
        }

        r
    }

    fn parse_gamepack(data: &[u8]) -> nom::IResult<&[u8], Vec<u16>> {
        let (i, v) = multi::many0(number::complete::le_u16)(&data[..])?;
        Ok((i, v))
    }

    pub fn load_gamepack(&mut self, file_path: &str) -> io::Result<()> {
        let (_, data) = GbaSystem::parse_gamepack(&(fs::read(file_path)?)[..]).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        self.pack[0..data.len()].copy_from_slice(&data);
        
        Ok(())
    }
}
