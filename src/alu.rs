use crate::gba::*;
use crate::decode_arm::*;
use crate::arm::ProgramStatus;
use std::fmt;

pub fn sign_extend(val: u32, bits: usize) -> i32 {
    if val & (1 << (bits - 1)) != 0 {
        let mut n_val = val;
        for n in bits..32 {
            n_val |= 1 << n;
        }

        n_val as i32
    } else {
        val as i32
    }
}

impl GbaSystem {
    fn update_flags(&mut self, ru: &(u32, bool), rs: &(i32, bool)) {
        self.cpsr.set(ProgramStatus::FLAG_N, (ru.0 & 0x80000000) != 0);
        self.cpsr.set(ProgramStatus::FLAG_Z, ru.0 == 0);
        self.cpsr.set(ProgramStatus::FLAG_C, ru.1);
        self.cpsr.set(ProgramStatus::FLAG_V, rs.1);
    }

    fn calc_shifter_operand(&self, shifter_operand: &DPShifterOperand) -> (u32, bool) {
        match shifter_operand {
            &DPShifterOperand::Immediate { rotate, immed } => {
                let s = (immed as u32).rotate_right(rotate as u32 * 2);
                (s, (s & 0x80000000) != 0)
            },
            &DPShifterOperand::ImmediateShift { immed, shift_type, rm } => {
                match shift_type {
                    ShiftType::ASR => {
                        let s = (self.read_register(rm) as i32).overflowing_shr(immed.into());
                        (s.0 as u32, s.1)
                    },
                    ShiftType::LSL => 
                        if immed == 0 {
                            (self.read_register(rm), self.cpsr.contains(ProgramStatus::FLAG_C))
                        } else {
                            self.read_register(rm).overflowing_shl(immed.into())
                        },
                    ShiftType::LSR => self.read_register(rm).overflowing_shr(immed.into()),
                    ShiftType::ROR => (self.read_register(rm).rotate_right(immed.into()), false),
                    ShiftType::RRX => {
                        let s = self.read_register(rm).rotate_right(immed.into());
                        let c = self.cpsr.contains(ProgramStatus::FLAG_C);
                        let nc = (s & 0x80000000) != 0;
                        ((s & 0x7FFFFFFF) | (if self.cpsr.contains(ProgramStatus::FLAG_C) { 0x80000000 } else { 0 }), nc)
                    }
                }
            },
            &DPShifterOperand::RegisterShift { rs, shift_type, rm } => {
                match shift_type {
                    ShiftType::ASR => {
                        let s = (self.read_register(rm) as i32).overflowing_shr(self.read_register(rs));
                        (s.0 as u32, s.1)
                    },
                    ShiftType::LSL => self.read_register(rm).overflowing_shl(self.read_register(rs)),
                    ShiftType::LSR => self.read_register(rm).overflowing_shr(self.read_register(rs)),
                    ShiftType::ROR => (self.read_register(rm).rotate_right(self.read_register(rs)), false),
                    ShiftType::RRX => {
                        let s = if self.cpsr.contains(ProgramStatus::FLAG_C) { 1 } else { 0 } |
                        self.read_register(rm).overflowing_shr(1).0;
                        let co = (self.read_register(rm) & 0x1) != 0;

                        (s, co)
                    }
                }
            }
        }
    }

    pub fn alu_perform(&mut self, op: &DataProcessingOpcode, update_flags: bool, rd: Register, rn: Register, shifter_operand: &DPShifterOperand) {
        let shifter = self.calc_shifter_operand(shifter_operand);

        match op {
            &DataProcessingOpcode::AND => {
                self.write_register(rd, self.read_register(rn) & shifter.0);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::EOR => {
                self.write_register(rd, self.read_register(rn) ^shifter.0);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::ORR => {
                self.write_register(rd, self.read_register(rn) | shifter.0);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::RSB => {
                let mut ru = shifter.0.borrowing_sub(self.read_register(rn), false);
                let rs = (shifter.0 as i32).borrowing_sub(self.read_register(rn) as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.write_register(rd, ru.0);
            },
            &DataProcessingOpcode::RSC => {
                let mut ru = shifter.0.borrowing_sub(self.read_register(rn), !self.cpsr.contains(ProgramStatus::FLAG_C));
                let rs = (shifter.0 as i32).borrowing_sub(self.read_register(rn) as i32, !self.cpsr.contains(ProgramStatus::FLAG_C));

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.write_register(rd, ru.0);
            },
            &DataProcessingOpcode::ADD => {
                let reg_val = if rn == Register::R15 {
                    self.read_register(rn) & 0xFFFFFFFC
                } else {
                    self.read_register(rn)
                };

                let ru = reg_val.carrying_add(shifter.0, false);
                let rs = (reg_val as i32).carrying_add(shifter.0 as i32, false);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.write_register(rd, ru.0);
            },
            &DataProcessingOpcode::ADC => {
                let ru = self.read_register(rn).carrying_add(shifter.0, self.cpsr.contains(ProgramStatus::FLAG_C));
                let rs = (self.read_register(rn) as i32).carrying_add(shifter.0 as i32, self.cpsr.contains(ProgramStatus::FLAG_C));

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.write_register(rd, ru.0);
            },
            &DataProcessingOpcode::SUB => {
                let mut ru = self.read_register(rn).borrowing_sub(shifter.0, false);
                let rs = (self.read_register(rn) as i32).borrowing_sub(shifter.0 as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.write_register(rd, ru.0);
            },
            &DataProcessingOpcode::SBC => {
                let mut ru = self.read_register(rn).borrowing_sub(shifter.0, !self.cpsr.contains(ProgramStatus::FLAG_C));
                let rs = (self.read_register(rn) as i32).borrowing_sub(shifter.0 as i32, !self.cpsr.contains(ProgramStatus::FLAG_C));

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.write_register(rd, ru.0);
            },
            &DataProcessingOpcode::TST => {
                let r = self.read_register(rn) & shifter.0;

                self.cpsr.set(ProgramStatus::FLAG_N, (r & 0x80000000) != 0);
                self.cpsr.set(ProgramStatus::FLAG_Z, r == 0);
                self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
            },
            &DataProcessingOpcode::TEQ => {
                let r = self.read_register(rn) ^ shifter.0;

                self.cpsr.set(ProgramStatus::FLAG_N, (r & 0x80000000) != 0);
                self.cpsr.set(ProgramStatus::FLAG_Z, r == 0);
                self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
            },
            &DataProcessingOpcode::CMP => {
                let mut ru = self.read_register(rn).borrowing_sub(shifter.0, false);
                let rs = (self.read_register(rn) as i32).borrowing_sub(shifter.0 as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                self.update_flags(&ru, &rs);
            },
            &DataProcessingOpcode::CMN => {
                let mut ru = self.read_register(rn).carrying_add(shifter.0, false);
                let rs = (self.read_register(rn) as i32).carrying_add(shifter.0 as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                self.update_flags(&ru, &rs);
            },
            &DataProcessingOpcode::MOV => {
                self.write_register(rd, shifter.0);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);

                        if !self.cpsr.contains(ProgramStatus::FLAG_T) {
                            self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                        }
                    }
                }
            },
            &DataProcessingOpcode::MVN => {
                self.write_register(rd, !shifter.0);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::BIC => {
                self.write_register(rd, self.read_register(rd) & !shifter.0);

                if update_flags {
                    // Special case for r15
                    if rd == Register::R15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::MUL => {
                self.write_register(rd, self.read_register(rd).overflowing_mul(shifter.0 as u32).0);

                self.cpsr.set(ProgramStatus::FLAG_N, (self.read_register(rd) & 0x80000000) != 0);
                self.cpsr.set(ProgramStatus::FLAG_Z, self.read_register(rd) == 0);
            }
        }
    }
}