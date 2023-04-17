use crate::bus::BusValue;
use crate::bus::BusWidth;
use crate::gba::*;
use crate::decode_arm::*;

use bitflags::BitFlags;
use bitflags::bitflags;
use std::convert::TryFrom;

pub enum Mode {
    User = 0b10000,
    FIQ = 0b10001,
    IRQ = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    Undefined = 0b11011,
    System = 0b11111,
}

impl TryFrom<u32> for Mode {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == Mode::User as u32 => Ok(Mode::User),
            x if x == Mode::FIQ as u32 => Ok(Mode::FIQ),
            x if x == Mode::IRQ as u32 => Ok(Mode::IRQ),
            x if x == Mode::Supervisor as u32 => Ok(Mode::Supervisor),
            x if x == Mode::Abort as u32 => Ok(Mode::Abort),
            x if x == Mode::Undefined as u32 => Ok(Mode::Undefined),
            x if x == Mode::System as u32 => Ok(Mode::System),
            _ => Err(()),
        }
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct ProgramStatus: u32 {
        const FLAG_N    = 0b10000000000000000000000000000000;
        const FLAG_Z    = 0b01000000000000000000000000000000;
        const FLAG_C    = 0b00100000000000000000000000000000;
        const FLAG_V    = 0b00010000000000000000000000000000;
        const FLAG_Q    = 0b00001000000000000000000000000000;
        const FLAG_J    = 0b00000001000000000000000000000000;
        const FLAG_GE_3 = 0b00000000000010000000000000000000;
        const FLAG_GE_2 = 0b00000000000001000000000000000000;
        const FLAG_GE_1 = 0b00000000000000100000000000000000;
        const FLAG_GE_0 = 0b00000000000000010000000000000000;
        const FLAG_E    = 0b00000000000000000000001000000000;
        const FLAG_A    = 0b00000000000000000000000100000000;
        const FLAG_I    = 0b00000000000000000000000010000000;
        const FLAG_F    = 0b00000000000000000000000001000000;
        const FLAG_T    = 0b00000000000000000000000000100000;
        const FLAG_MODE = 0b00000000000000000000000000011111;
    }
}

impl ProgramStatus {
    pub fn get_mode(&self) -> Mode {
        (self.bits() & ProgramStatus::FLAG_MODE.bits()).try_into().unwrap()
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.0.bits = (self.0.bits & !ProgramStatus::FLAG_MODE.bits()) | m as u32;
    }
}

impl GbaSystem {
    pub fn test_condition(&self, c: Condition) -> bool {
        match c {
            Condition::AL |
            Condition::UNC => true,
            Condition::EQ => self.cpsr.contains(ProgramStatus::FLAG_Z),
            Condition::NE => !self.cpsr.contains(ProgramStatus::FLAG_Z),
            Condition::CSHS => self.cpsr.contains(ProgramStatus::FLAG_C),
            Condition::CCLO => !self.cpsr.contains(ProgramStatus::FLAG_C),
            Condition::MI => self.cpsr.contains(ProgramStatus::FLAG_N),
            Condition::PL => !self.cpsr.contains(ProgramStatus::FLAG_N),
            Condition::VS => self.cpsr.contains(ProgramStatus::FLAG_V),
            Condition::VC => !self.cpsr.contains(ProgramStatus::FLAG_V),
            Condition::HI => self.cpsr.contains(ProgramStatus::FLAG_C) && !self.cpsr.contains(ProgramStatus::FLAG_Z),
            Condition::LS => !self.cpsr.contains(ProgramStatus::FLAG_C) || self.cpsr.contains(ProgramStatus::FLAG_Z),
            Condition::GE => self.cpsr.contains(ProgramStatus::FLAG_N) == self.cpsr.contains(ProgramStatus::FLAG_V),
            Condition::LT => self.cpsr.contains(ProgramStatus::FLAG_N) != self.cpsr.contains(ProgramStatus::FLAG_V),
            Condition::GT => !self.cpsr.contains(ProgramStatus::FLAG_Z) && (self.cpsr.contains(ProgramStatus::FLAG_N) == self.cpsr.contains(ProgramStatus::FLAG_V)),
            Condition::LE => self.cpsr.contains(ProgramStatus::FLAG_Z) && (self.cpsr.contains(ProgramStatus::FLAG_N) != self.cpsr.contains(ProgramStatus::FLAG_V)),
        }
    }

    pub fn load_mode_spsr(&mut self) {
        match self.cpsr.get_mode() {
            Mode::User |
            Mode::System => (),
            Mode::FIQ => self.cpsr = self.spsr_fiq,
            Mode::IRQ => self.cpsr = self.spsr_irq,
            Mode::Supervisor => self.cpsr = self.spsr_svc,
            Mode::Abort => self.cpsr = self.spsr_abt,
            Mode::Undefined => self.cpsr = self.spsr_und,
        }
    }

    fn is_privileged(&self) -> bool {
        match self.cpsr.get_mode() {
            Mode::User => false,
            _ => true,
        }
    }

    fn compute_masked_state(&self, state: ProgramStatus, spsr: bool, mask: MSRMask, val: u32) -> ProgramStatus {
        const UNALLOC_MASK:u32  = 0x06F0FC00;
        const USER_MASK:u32     = 0xF80F0200;
        const PRIV_MASK:u32     = 0x000001DF;
        const STATE_MASK:u32    = 0x01000020;

        let mut state_mask = mask.get_mask();

        if spsr {
            state_mask &= USER_MASK | PRIV_MASK | STATE_MASK;
        } else {
            if self.is_privileged() {
                if (val & STATE_MASK) != 0 {
                    state_mask = 0;
                } else {
                    state_mask &= USER_MASK | PRIV_MASK;
                }
            } else {
                state_mask &= USER_MASK;
            }
        }

        ProgramStatus::from_bits_retain((state.bits() & !state_mask) | (val & state_mask))
    }

    pub fn get_spsr(&self) -> Option<ProgramStatus> {
        match self.cpsr.get_mode() {
            Mode::User |
            Mode::System => None,
            Mode::Abort => Some(self.spsr_abt),
            Mode::IRQ => Some(self.spsr_irq),
            Mode::FIQ => Some(self.spsr_fiq),
            Mode::Supervisor => Some(self.spsr_svc),
            Mode::Undefined => Some(self.spsr_und),
        }
    }

    pub fn set_spsr(&mut self, val: ProgramStatus) -> Result<(), ()> {
        match self.cpsr.get_mode() {
            Mode::User |
            Mode::System => Err(()),
            Mode::Abort => { self.spsr_abt = val; Ok(()) },
            Mode::IRQ => { self.spsr_irq = val; Ok(()) },
            Mode::FIQ => { self.spsr_fiq = val; Ok(()) },
            Mode::Supervisor => { self.spsr_svc = val; Ok(()) },
            Mode::Undefined => { self.spsr_und = val; Ok(()) },
        }
    }

    pub fn instruction_size(&self) -> u32 {
        if self.cpsr.contains(ProgramStatus::FLAG_T) {
            2
        } else {
            4
        }
    }

    pub fn execute(&mut self, inst: &ArmInstruction) -> u32 {
        match inst {
            &ArmInstruction::DataProcessing { c, op, s, rn, rd, shifter_operand } => {
                if self.test_condition(c) {
                    self.alu_perform(&op, s, rd as usize, rn as usize, &shifter_operand);
                }

                4
            },
            &ArmInstruction::LoadStore { c, pre_indexed, add_offset, byte_access, w, load, rn, rd, shifter_operand } => {
                if self.test_condition(c) {
                    // Determine address
                    let mut adr: u32 = match shifter_operand {
                        LSShifterOperand::Immediate { immed } => immed as u32,
                        LSShifterOperand::ImmediateShift { immed, shift_type, rm } => {
                            match shift_type {
                                ShiftType::LSL => self.r[rm].overflowing_shl(immed.into()).0,
                                ShiftType::LSR => if immed == 0 {
                                    0
                                } else {
                                    self.r[rm].overflowing_shr(immed.into()).0
                                },
                                ShiftType::ASR => {
                                    if immed == 0 {
                                        if (self.r[rm] & 0x80000000) != 0 {
                                            0xFFFFFFFF
                                        } else {
                                            0
                                        }
                                    } else {
                                        (self.r[rm] as i32).overflowing_shr(immed.into()).0 as u32
                                    }
                                },
                                ShiftType::ROR => {
                                    self.r[rm].rotate_right(immed.into())
                                },
                                ShiftType::RRX => {
                                    (if self.cpsr.contains(ProgramStatus::FLAG_C) {
                                        0x80000000
                                    } else {
                                        0
                                    }) | 
                                    self.r[rm].overflowing_shr(immed.into()).0
                                }
                            }
                        }
                    };
                    
                    if add_offset {
                        adr = self.r[rn as usize].wrapping_add(adr);
                    } else {
                        adr = self.r[rn as usize].wrapping_sub(adr);
                    }

                    if !pre_indexed {
                        self.r[rn as usize] = adr;
                    }

                    // Switch load or store
                    if load {
                        if byte_access {
                            match self.read_bus_byte(adr) {
                                Some(d) => self.r[rd as usize] = d as u32,
                                None => panic!("Access violation")
                            };
                        } else {
                            match self.read_bus_word(adr) {
                                Some(d) => {
                                    if rd == 15 {
                                        self.r[rd as usize] = d & 0xFFFFFFE;
                                        self.cpsr.set(ProgramStatus::FLAG_T, d & 0x1 != 0);
                                    } else {
                                        self.r[rd as usize] = d;
                                    }
                                }
                                None => panic!("Access violation")
                            };
                        }
                    } else {
                        if byte_access {
                            match self.write_bus(adr, BusValue::Byte((self.r[rd as usize] & 0xFF) as u8)) {
                                Ok(_) => (),
                                Err(_) => panic!("Access violation")
                            }
                        } else {
                            match self.write_bus(adr, BusValue::Word(self.r[rd as usize])) {
                                Ok(_) => (),
                                Err(_) => panic!("Access violation")
                            }
                        }
                    }
                }

                4
            },
            &ArmInstruction::LoadStoreStatus { c, r, op } => {
                if self.test_condition(c) {
                    match op {
                        LoadStoreStatusOperation::StatusRegisterToRegister { rd } => {
                            if r {
                                match self.get_spsr() {
                                    Some(v) => self.r[rd] = v.bits(),
                                    None => (),
                                }
                            } else {
                                self.r[rd] = self.cpsr.bits();

                            }
                        },
                        LoadStoreStatusOperation::RegisterToStatusRegister { mask, rm } => {
                            if r {
                                match self.get_spsr() {
                                    Some(v) => {
                                        self.set_spsr(self.compute_masked_state(v, true, mask, self.r[rm])).unwrap();
                                    },
                                    _ => (),
                                }
                            } else {
                                self.cpsr = self.compute_masked_state(self.cpsr, true, mask, self.r[rm]);
                            }
                        },
                        LoadStoreStatusOperation::ImmediateToStatusRegister { mask, rot_imm, immed } => {
                            let set_val = (immed as u32).rotate_right(rot_imm as u32);

                            if r {
                                match self.get_spsr() {
                                    Some(v) => {
                                        self.set_spsr(self.compute_masked_state(v, true, mask, set_val)).unwrap();
                                    },
                                    _ => (),
                                }
                            } else {
                                self.cpsr = self.compute_masked_state(self.cpsr, true, mask, set_val);
                            }
                        },
                    }
                }

                4
            }
            &ArmInstruction::Branch { c, op } => {
                if self.test_condition(c) {
                    match op {
                        BranchOperation::BranchImmed { offset } => self.r[15] = self.r[15].wrapping_add_signed(offset).wrapping_add(4),
                        BranchOperation::BranchLinkImmed { offset } => {
                            self.r[14] = self.r[15];
                            self.r[15] = self.r[15].wrapping_add_signed(offset).wrapping_add(self.instruction_size());
                        },
                        BranchOperation::BranchExchangeThumb { rm } => {
                            self.r[15] = self.r[rm] & 0xFFFFFFFE;
                            self.r[15] = self.r[15].wrapping_add(self.instruction_size());
                            self.cpsr.set(ProgramStatus::FLAG_T, self.r[rm] & 0x1 != 0);
                        },
                        BranchOperation::BranchExchangeLinkThumb { rm } => {
                            self.r[14] = self.r[15];
                            self.r[15] = self.r[rm] & 0xFFFFFFFE;
                            self.r[15] = self.r[15].wrapping_add(self.instruction_size());
                            self.cpsr.set(ProgramStatus::FLAG_T, self.r[rm] & 0x1 != 0);
                        }
                        _ => panic!("Unimplemented branch instruction!"),
                    }                   
                }

                4
            },
            _ => panic!("Unimplemented instruction!")
        }
    }
}