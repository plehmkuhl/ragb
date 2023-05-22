use crate::alu::sign_extend;
use crate::bus::BusValue;
use crate::bus::BusWidth;
use crate::gba::*;
use crate::decode_arm::*;

use bitflags::BitFlags;
use bitflags::bitflags;
use std::fmt;
use std::convert::TryFrom;

pub enum Mode {
    User,
    FIQ,
    IRQ,
    Supervisor,
    Abort,
    Undefined,
    System,
}

impl TryFrom<u32> for Mode {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            0b10000 => Ok(Mode::User),
            0b10001 => Ok(Mode::FIQ),
            0b10010 => Ok(Mode::IRQ),
            0b10011 => Ok(Mode::Supervisor),
            0b10111 => Ok(Mode::Abort),
            0b11011 => Ok(Mode::Undefined),
            0b11111 => Ok(Mode::System),
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
        (self.bits() & ProgramStatus::FLAG_MODE.bits()).try_into().unwrap_or(Mode::User)
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.0.bits = (self.0.bits & !ProgramStatus::FLAG_MODE.bits()) | match m {
            Mode::User => 0b10000,
            Mode::FIQ => 0b10001,
            Mode::IRQ => 0b10010,
            Mode::Supervisor => 0b10011,
            Mode::Abort => 0b10111,
            Mode::Undefined => 0b11011,
            Mode::System => 0b11111,
        };
    }
}

impl fmt::Display for ProgramStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:08x} [{}{}{}{}{}{}{}]", self.bits() 
            ,if self.contains(ProgramStatus::FLAG_N) { "N" } else { "-" }
            ,if self.contains(ProgramStatus::FLAG_Z) { "Z" } else { "-" }
            ,if self.contains(ProgramStatus::FLAG_C) { "C" } else { "-" }
            ,if self.contains(ProgramStatus::FLAG_V) { "V" } else { "-" }
            ,if self.contains(ProgramStatus::FLAG_I) { "I" } else { "-" }
            ,if self.contains(ProgramStatus::FLAG_F) { "F" } else { "-" }
            ,if self.contains(ProgramStatus::FLAG_T) { "T" } else { "-" })
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
                    self.alu_perform(&op, s, rd, rn, &shifter_operand);

                    if rd == Register::R15 {
                        let val = self.read_register(rd);
                        self.write_register(rd, val & 0xFFFFFFFE);
                        self.cpsr.set(ProgramStatus::FLAG_T, (val & 0x1) != 0 );
                    }
                }

                4
            },
            &ArmInstruction::LoadStore { c, pre_indexed, add_offset, width, w, load, rn, rd, shifter_operand } => {
                if self.test_condition(c) {
                    // Determine address
                    let mut adr: u32 = match shifter_operand {
                        LSShifterOperand::Immediate { immed } => immed as u32,
                        LSShifterOperand::ImmediateShift { immed, shift_type, rm } => {
                            match shift_type {
                                ShiftType::LSL => self.read_register(rm).overflowing_shl(immed.into()).0,
                                ShiftType::LSR => if immed == 0 {
                                    0
                                } else {
                                    self.read_register(rm).overflowing_shr(immed.into()).0
                                },
                                ShiftType::ASR => {
                                    if immed == 0 {
                                        if (self.read_register(rm) & 0x80000000) != 0 {
                                            0xFFFFFFFF
                                        } else {
                                            0
                                        }
                                    } else {
                                        (self.read_register(rm) as i32).overflowing_shr(immed.into()).0 as u32
                                    }
                                },
                                ShiftType::ROR => {
                                    self.read_register(rm).rotate_right(immed.into())
                                },
                                ShiftType::RRX => {
                                    (if self.cpsr.contains(ProgramStatus::FLAG_C) {
                                        0x80000000
                                    } else {
                                        0
                                    }) | 
                                    self.read_register(rm).overflowing_shr(immed.into()).0
                                }
                            }
                        }
                    };

                    let addr_base = if rn == Register::R15 && self.cpsr.contains(ProgramStatus::FLAG_T) {
                        self.read_register(rn) & 0xFFFFFFFC
                    } else {
                        self.read_register(rn)
                    };
                    
                    if add_offset {
                        adr = addr_base.wrapping_add(adr);
                    } else {
                        adr = addr_base.wrapping_sub(adr);
                    }
                    
                    if !pre_indexed {
                        self.write_register(rn, adr);
                    }

                    // Switch load or store
                    if load {
                        //println!("Read bus: {:08x}", adr);

                        match width {
                            BusWidth::Byte => {
                                match self.read_bus_byte(adr) {
                                    Some(d) => self.write_register(rd, d as u32),
                                    None => panic!("Access violation")
                                };
                            },
                            BusWidth::HalfWord => {
                                match self.read_bus_half_word(adr) {
                                    Some(d) => self.write_register(rd, d as u32),
                                    None => panic!("Access violation")
                                };
                            },
                            BusWidth::Word => {
                                match self.read_bus_word(adr) {
                                    Some(d) => {
                                        if rd == Register::R15 {
                                            self.write_register(rd, d & 0xFFFFFF);
                                            self.cpsr.set(ProgramStatus::FLAG_T, d & 0x1 != 0);
                                        } else {
                                            self.write_register(rd, d);
                                        }
                                    }
                                    None => panic!("Access violation")
                                };
                            }
                        }
                    } else {
                        //println!("Write bus: {:08x}", adr);

                        match width {
                            BusWidth::Byte => {
                                match self.write_bus(adr, BusValue::Byte((self.read_register(rd) & 0xFF) as u8)) {
                                    Ok(_) => (),
                                    Err(_) => panic!("Access violation")
                                }
                            },
                            BusWidth::HalfWord => {
                                match self.write_bus(adr, BusValue::HalfWord((self.read_register(rd) & 0xFFFF) as u16)) {
                                    Ok(_) => (),
                                    Err(_) => panic!("Access violation")
                                }
                            },
                            BusWidth::Word => {
                                match self.write_bus(adr, BusValue::Word(self.read_register(rd))) {
                                    Ok(_) => (),
                                    Err(_) => panic!("Access violation")
                                }
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
                                    Some(v) => self.write_register(rd, v.bits()),
                                    None => (),
                                }
                            } else {
                                self.write_register(rd, self.cpsr.bits());
                            }
                        },
                        LoadStoreStatusOperation::RegisterToStatusRegister { mask, rm } => {
                            if r {
                                match self.get_spsr() {
                                    Some(v) => {
                                        self.set_spsr(self.compute_masked_state(v, true, mask, self.read_register(rm))).unwrap();
                                    },
                                    _ => (),
                                }
                            } else {
                                self.cpsr = self.compute_masked_state(self.cpsr, true, mask, self.read_register(rm));
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
            &ArmInstruction::LoadStoreMultiple {c, exclude_first_word, upwards, update_base, load_usermode, load, rn, register_list} => {
                if self.test_condition(c) {
                    //let start_address: u32 = self.read_register(rn); 
                    let (start_address, end_address, base) = 
                        if !exclude_first_word && upwards {
                            (
                                self.read_register(rn), 
                                self.read_register(rn).wrapping_add(register_list.count() * 4) - 4,
                                self.read_register(rn).wrapping_add(register_list.count() * 4)
                            )
                        } else if exclude_first_word && upwards {
                            (
                                self.read_register(rn).wrapping_add(4), 
                                self.read_register(rn).wrapping_add(register_list.count() * 4),
                                self.read_register(rn).wrapping_add(register_list.count() * 4)
                            )
                        } else if !exclude_first_word && !upwards {
                            (
                                self.read_register(rn).wrapping_sub(register_list.count() * 4) + 4, 
                                self.read_register(rn),
                                self.read_register(rn).wrapping_sub(register_list.count() * 4)
                            )
                        } else {
                            (
                                self.read_register(rn).wrapping_sub(register_list.count() * 4), 
                                self.read_register(rn).wrapping_sub(4),
                                self.read_register(rn).wrapping_sub(register_list.count() * 4)
                            )
                        };

                    if update_base {
                        self.write_register(rn, base);
                    }
                    
                    let mut address = start_address;
                    for n in register_list {
                        let reg = match n {
                            RegisterList::FLAG_R0 => Register::R0,
                            RegisterList::FLAG_R1 => Register::R1,
                            RegisterList::FLAG_R2 => Register::R2,
                            RegisterList::FLAG_R3 => Register::R3,
                            RegisterList::FLAG_R4 => Register::R4,
                            RegisterList::FLAG_R5 => Register::R5,
                            RegisterList::FLAG_R6 => Register::R6,
                            RegisterList::FLAG_R7 => Register::R7,
                            RegisterList::FLAG_R8 => Register::R8,
                            RegisterList::FLAG_R9 => Register::R9,
                            RegisterList::FLAG_R10 => Register::R10,
                            RegisterList::FLAG_R11 => Register::R11,
                            RegisterList::FLAG_R12 => Register::R12,
                            RegisterList::FLAG_R13 => Register::R13,
                            RegisterList::FLAG_R14 => Register::R14,
                            RegisterList::FLAG_R15 => Register::R15,
                            _ => unreachable!(),
                        };

                        if load {
                            if let Some(val) = self.read_bus_word(address) {
                                //println!("Read bus: {:08x} -> ({}) {:08x}", address, reg, val);

                                if reg == Register::R15 {
                                    self.write_register(Register::R15, val & 0xFFFFFFFE);
                                    self.cpsr.set(ProgramStatus::FLAG_T, (val & 0x1) != 0 );
                                } else {
                                    self.write_register(reg, val);
                                }
                            }
                        } else {
                            //println!("Write bus: {:08x} -> ({}) {:08x}", address, reg, self.read_register(reg));
                            self.write_bus(address, BusValue::Word(self.read_register(reg)));
                        }

                        address = address.wrapping_add(4);
                    }
                }

                4
            },
            &ArmInstruction::Branch { c, op } => {
                if self.test_condition(c) {
                    match op {
                        BranchOperation::BranchImmed { offset } => {
                            self.write_register(Register::R15, self.read_register(Register::R15).wrapping_add_signed(offset));
                        },
                        BranchOperation::BranchLinkImmed { offset, lr_correct } => {
                            self.write_register(Register::R14, self.read_register(Register::R15).wrapping_sub(self.instruction_size()).wrapping_add(lr_correct as u32));
                            if self.cpsr.contains(ProgramStatus::FLAG_T) {
                                self.write_register(Register::R14, self.read_register(Register::R14) | 1);
                            }
                            self.write_register(Register::R15, self.read_register(Register::R15).wrapping_add_signed(offset));
                        },
                        BranchOperation::BranchExchangeThumb { rm } => {
                            self.write_register(Register::R15, self.read_register(rm) & 0xFFFFFFFE);
                            self.cpsr.set(ProgramStatus::FLAG_T, self.read_register(rm) & 0x1 != 0);
                        },
                        BranchOperation::BranchExchangeLinkThumb { rm } => {
                            self.write_register(Register::R14, self.read_register(Register::R15).wrapping_sub(self.instruction_size()));
                            self.write_register(Register::R15, self.read_register(rm) & 0xFFFFFFFE);
                            self.cpsr.set(ProgramStatus::FLAG_T, self.read_register(rm) & 0x1 != 0);
                        }
                        BranchOperation::BranchExchangeLinkThumbImmed { offset } => {
                            self.write_register(Register::R14, self.read_register(Register::R15).wrapping_sub(self.instruction_size()));
                            self.write_register(Register::R15, self.read_register(Register::R15).wrapping_add_signed(offset));
                            self.cpsr.set(ProgramStatus::FLAG_T, false);
                        }
                        _ => panic!("Unimplemented branch instruction!"),
                    }                   
                }

                4
            },
            &ArmInstruction::BranchLinkPrefix { offset } => {
                self.write_register(Register::R14, 
                    self.read_register(Register::R15)
                        .wrapping_add(
                            sign_extend((offset as u32) << 12, 22) as u32
                        ));
                4
            },
            &ArmInstruction::BranchLinkSuffix { op } => {
                match op {
                    BranchOperation::BranchLinkImmed { offset, lr_correct } => {
                        let pc = self.read_register(Register::R14).wrapping_add((offset << 1) as u32);
                        self.write_register(Register::R14, self.read_register(Register::R15).wrapping_sub(self.instruction_size()) | 1);
                        self.write_register(Register::R15, pc);
                    },
                    BranchOperation::BranchExchangeLinkThumbImmed { offset } => {
                        let pc = self.read_register(Register::R14).wrapping_add((offset << 1) as u32) & 0xFFFFFFFC;
                        self.write_register(Register::R14, self.read_register(Register::R15).wrapping_sub(self.instruction_size()) | 1);
                        self.write_register(Register::R15, pc);
        
                        self.cpsr.set(ProgramStatus::FLAG_T, false);
                    },
                    _ => unreachable!()
                }
                4
            }
            _ => panic!("Unimplemented instruction!")
        }
    }
}