use crate::{
    arm::*, 
    bus::BusValue, 
    decode_arm::{InstructionTableEntry, generate_arm_instruction_table},
    decode_thumb::{ThumbInstructionTableEntry, generate_thumb_instruction_table}
};

pub const CPU_TICKS_PER_SECOND: u32 = 16780000;

pub struct GbaSystem {
    // Memory (32-Bit bus)
    pub bios:Vec<u32>, 
    pub iwram:Vec<u32>,
    pub oam:Vec<u32>,
    pub io:Vec<u32>,

    // Memory (16-Bit bus)
    pub ewram:Vec<u16>,
    pub vram:Vec<u16>,
    pub pram:Vec<u16>,
    pub pack:Vec<u16>,

    // Memory (8-Bit bus)
    pub sram:Vec<u8>,

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

    // CPU
    pub instruction_table: Vec<InstructionTableEntry>,
    pub thumb_instruction_table: Vec<ThumbInstructionTableEntry>,
}

impl GbaSystem {
    pub fn new() -> GbaSystem {
        GbaSystem {
            bios: vec![0; 4096],
            iwram: vec![0; 8192],
            oam: vec![0; 256],
            io: vec![0; 256],
            ewram: vec![0; 131072],
            vram: vec![0; 49152],
            pram: vec![0; 512],
            pack: vec![0; 16777216],
            sram: vec![0; 65536],
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
            instruction_table: generate_arm_instruction_table(),
            thumb_instruction_table: generate_thumb_instruction_table(),
        }
    }

    pub fn reset(&mut self) {
        self.r_svc[1] = self.r[15];
        self.spsr_svc = self.cpsr;

        self.cpsr.set_mode(Mode::Supervisor);
        self.cpsr.set(ProgramStatus::FLAG_I, true);
        self.cpsr.set(ProgramStatus::FLAG_F, true);
        self.cpsr.set(ProgramStatus::FLAG_T, false);

        self.r[15] = 0;
    }

    pub fn emulate(&mut self) -> u32 {
        let adr = self.r[15];
        let instruction_size = self.instruction_size();
        let decoded = 
        // Thumb mode
        if self.cpsr.contains(ProgramStatus::FLAG_T) {
            match self.read_bus_half_word(adr) {
                Some(inst) => self.decode_thumb_instruction(inst),
                _ => None,
            }
        // Arm Mode
        } else {
            match self.read_bus_word(adr) {
                Some(inst) => self.decode_instruction(inst),
                _ => None,
            }
        };

        match decoded {
            Some(inst) => {
                //println!("{:#08x} {} {}", adr, if self.cpsr.contains(ProgramStatus::FLAG_T) { "T" } else { " " }, inst);

                self.r[15] = self.r[15].wrapping_add(instruction_size);
                let ticks = self.execute(&inst);
            },
            None => panic!("Invalid instruction!"),
        }

        4
    }
}