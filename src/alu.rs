use crate::gba::*;
use crate::decode_arm::*;
use crate::arm::ProgramStatus;
use std::fmt;

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
                        let s = (self.r[rm] as i32).overflowing_shr(immed.into());
                        (s.0 as u32, s.1)
                    },
                    ShiftType::LSL => 
                        if immed == 0 {
                            (self.r[rm], self.cpsr.contains(ProgramStatus::FLAG_C))
                        } else {
                            self.r[rm].overflowing_shl(immed.into())
                        },
                    ShiftType::LSR => self.r[rm].overflowing_shr(immed.into()),
                    ShiftType::ROR => (self.r[rm].rotate_right(immed.into()), false),
                    ShiftType::RRX => {
                        let s = self.r[rm].rotate_right(immed.into());
                        let c = self.cpsr.contains(ProgramStatus::FLAG_C);
                        let nc = (s & 0x80000000) != 0;
                        ((s & 0x7FFFFFFF) | (if self.cpsr.contains(ProgramStatus::FLAG_C) { 0x80000000 } else { 0 }), nc)
                    }
                }
            },
            &DPShifterOperand::RegisterShift { rs, shift_type, rm } => {
                match shift_type {
                    ShiftType::ASR => {
                        let s = (self.r[rm] as i32).overflowing_shr(self.r[rs]);
                        (s.0 as u32, s.1)
                    },
                    ShiftType::LSL => self.r[rm].overflowing_shl(self.r[rs]),
                    ShiftType::LSR => self.r[rm].overflowing_shr(self.r[rs]),
                    ShiftType::ROR => (self.r[rm].rotate_right(self.r[rs]), false),
                    ShiftType::RRX => {
                        let s = if self.cpsr.contains(ProgramStatus::FLAG_C) { 1 } else { 0 } |
                            self.r[rm].overflowing_shr(1).0;
                        let co = (self.r[rm] & 0x1) != 0;

                        (s, co)
                    }
                }
            }
        }
    }

    pub fn alu_perform(&mut self, op: &DataProcessingOpcode, update_flags: bool, rd: usize, rn: usize, shifter_operand: &DPShifterOperand) {
        let shifter = self.calc_shifter_operand(shifter_operand);

        match op {
            &DataProcessingOpcode::AND => {
                self.r[rd] = self.r[rn] & shifter.0;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.r[rd] & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.r[rd] == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::EOR => {
                self.r[rd] = self.r[rn] ^ shifter.0;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.r[rd] & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.r[rd] == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::ORR => {
                self.r[rd] = self.r[rn] | shifter.0;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.r[rd] & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.r[rd] == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::RSB => {
                let mut ru = shifter.0.borrowing_sub(self.r[rn], false);
                let rs = (shifter.0 as i32).borrowing_sub(self.r[rn] as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.r[rd] = ru.0;
            },
            &DataProcessingOpcode::RSC => {
                let mut ru = shifter.0.borrowing_sub(self.r[rn], !self.cpsr.contains(ProgramStatus::FLAG_C));
                let rs = (shifter.0 as i32).borrowing_sub(self.r[rn] as i32, !self.cpsr.contains(ProgramStatus::FLAG_C));

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.r[rd] = ru.0;
            },
            &DataProcessingOpcode::ADD => {
                let ru = self.r[rn].carrying_add(shifter.0, false);
                let rs = (self.r[rn] as i32).carrying_add(shifter.0 as i32, false);

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.r[rd] = ru.0;
            },
            &DataProcessingOpcode::ADC => {
                let ru = self.r[rn].carrying_add(shifter.0, self.cpsr.contains(ProgramStatus::FLAG_C));
                let rs = (self.r[rn] as i32).carrying_add(shifter.0 as i32, self.cpsr.contains(ProgramStatus::FLAG_C));

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.r[rd] = ru.0;
            },
            &DataProcessingOpcode::SUB => {
                let mut ru = self.r[rn].borrowing_sub(shifter.0, false);
                let rs = (self.r[rn] as i32).borrowing_sub(shifter.0 as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.r[rd] = ru.0;
            },
            &DataProcessingOpcode::SBC => {
                let mut ru = self.r[rn].borrowing_sub(shifter.0, !self.cpsr.contains(ProgramStatus::FLAG_C));
                let rs = (self.r[rn] as i32).borrowing_sub(shifter.0 as i32, !self.cpsr.contains(ProgramStatus::FLAG_C));

                // Flip borrow flag
                ru.1 = !ru.1;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.update_flags(&ru, &rs);
                    }
                }

                self.r[rd] = ru.0;
            },
            &DataProcessingOpcode::TST => {
                let r = self.r[rn] & shifter.0;

                self.cpsr.set(ProgramStatus::FLAG_N, (r & 0x80000000) != 0);
                self.cpsr.set(ProgramStatus::FLAG_Z, r == 0);
                self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
            },
            &DataProcessingOpcode::TEQ => {
                let r = self.r[rn] ^ shifter.0;

                self.cpsr.set(ProgramStatus::FLAG_N, (r & 0x80000000) != 0);
                self.cpsr.set(ProgramStatus::FLAG_Z, r == 0);
                self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
            },
            &DataProcessingOpcode::CMP => {
                let mut ru = self.r[rn].borrowing_sub(shifter.0, false);
                let rs = (self.r[rn] as i32).borrowing_sub(shifter.0 as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                self.update_flags(&ru, &rs);
            },
            &DataProcessingOpcode::CMN => {
                let mut ru = self.r[rn].carrying_add(shifter.0, false);
                let rs = (self.r[rn] as i32).carrying_add(shifter.0 as i32, false);

                // Flip borrow flag
                ru.1 = !ru.1;

                self.update_flags(&ru, &rs);
            },
            &DataProcessingOpcode::MOV => {
                self.r[rd] = shifter.0;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.r[rd] & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.r[rd] == 0);

                        if !self.cpsr.contains(ProgramStatus::FLAG_T) {
                            self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                        }
                    }
                }
            },
            &DataProcessingOpcode::MVN => {
                self.r[rd] = !shifter.0;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.r[rd] & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.r[rd] == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
            &DataProcessingOpcode::BIC => {
                self.r[rd] = self.r[rd] & !shifter.0;

                if update_flags {
                    // Special case for r15
                    if rd == 15 {
                        self.load_mode_spsr();
                    } else {
                        self.cpsr.set(ProgramStatus::FLAG_N, (self.r[rd] & 0x80000000) != 0);
                        self.cpsr.set(ProgramStatus::FLAG_Z, self.r[rd] == 0);
                        self.cpsr.set(ProgramStatus::FLAG_C, shifter.1);
                    }
                }
            },
        }
    }
}