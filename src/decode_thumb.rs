use crate::gba::*;
use crate::alu::sign_extend;
use crate::decode_arm::{ArmInstruction, Condition, DataProcessingOpcode, DPShifterOperand, ShiftType, BranchOperation, RegisterList};
use crate::bus::BusWidth;

use nom::{
    IResult,
    bits,
    bits::complete::{bool, tag, take},
    sequence::tuple, combinator::fail,
};
use std::ops::Shl;
use std::{fmt, ops::Shr};

pub type ThumbInstructionTableEntry = (u16, u16, for<'a> fn((&'a[u8], usize)) -> IResult<(&'a[u8], usize), ArmInstruction>);

const INSTRUCTION_TABLE: &'static [ThumbInstructionTableEntry] = &[
    (
        // Shift by immediate
        0b1110000000000000,
        0b0000000000000000,
        decode_shift_by_immediate
    ),
    (
        // Add/subtract register
        0b1111110000000000,
        0b0001100000000000,
        decode_add_subtract_register
    ),
    (
        // Add/subtract immediate
        0b1111110000000000,
        0b0001110000000000,
        decode_add_subtract_immediate
    ),
    (
        // Add/subtract/compare/move immediate
        0b1110000000000000,
        0b0010000000000000,
        decode_add_subtract_compare_move_immediate
    ),
    (
        // Data-processing register
        0b1111110000000000,
        0b0100000000000000,
        decode_data_processing_register
    ),
    (
        // Special data processing
        0b1111110000000000,
        0b0100010000000000,
        decode_special_data_processing
    ),
    (
        // Branch/exchange instruction set
        0b1111111100000000,
        0b0100011100000000,
        decode_branch_exchange
    ),
    (
        // Load from literal pool
        0b1111100000000000,
        0b0100100000000000,
        decode_load_from_literal_pool
    ),
    (
        // Load/store register offset
        0b1111000000000000,
        0b0101000000000000,
        decode_load_store_register_offset
    ),
    (
        // Load/store word/byte immediate offset
        0b1110000000000000,
        0b0110000000000000,
        decode_load_store_word_byte_immediate_offset
    ),
    (
        // Load/store halfword immediate offset
        0b1111000000000000,
        0b1000000000000000,
        decode_load_store_halfword_immediate_offset
    ),
    (
        // Load/store from stack
        0b1111000000000000,
        0b1001000000000000,
        decode_load_store_from_stack
    ),
    (
        // Add to SP or PC
        0b1111000000000000,
        0b1010000000000000,
        decode_add_to_sp_or_pc
    ),
    (
        // Adjust stack pointer
        0b1111111100000000,
        0b1011000000000000,
        decode_adjust_stack_pointer
    ),
    (
        // Push/pop register list
        0b1111011000000000,
        0b1011010000000000,
        decode_push_pop,
    ),
    (
        // Load/store multiple
        0b1111000000000000,
        0b1100000000000000,
        decode_load_store_multiple
    ),
    (
        // Conditional branch
        0b1111000000000000,
        0b1101000000000000,
        decode_branch_conditional
    ),
    (
        // Software interrupt
        0b1111111100000000,
        0b1101111100000000,
        decode_invalid
    ),
    (
        // Unconditional branch
        0b1111100000000000,
        0b1110000000000000,
        decode_unconditional_branch
    ),
    (
        //BLX suffix
        0b1111100000000001,
        0b1110100000000000,
        decode_blx_suffix
    ),
    (
        //BL/BLX prefix
        0b1111100000000000,
        0b1111000000000000,
        decode_bl_blx_prefix
    ),
    (
        //BL suffix
        0b1111100000000000,
        0b1111100000000000,
        decode_bl_suffix
    ),
];

fn decode_shift_by_immediate<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opcode, immed, rm, rd)): ((&[u8], usize), (u8, u8, u8, u8, u8)) = 
    tuple(
        (
            tag(0b000, 3usize),
            take(2usize),
            take(5usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::DataProcessing { 
        c: Condition::AL, 
        op: DataProcessingOpcode::MOV, 
        s: true, 
        rn: Register::try_from(rd).unwrap(), 
        rd: Register::try_from(rd).unwrap(), 
        shifter_operand: DPShifterOperand::ImmediateShift { 
            immed: immed, 
            shift_type: match opcode {
                0b00 => ShiftType::LSL,
                0b01 => ShiftType::LSR,
                0b10 => ShiftType::ASR,
                _ => ShiftType::ROR,
            }, 
            rm: Register::try_from(rm).unwrap() 
        } 
    }))
}

fn decode_add_subtract_register<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, sub, rm, rn, rd)): ((&[u8], usize), (u8, bool, u8, u8, u8)) = 
    tuple(
        (
            tag(0b000110, 6usize),
            bool,
            take(3usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    if sub {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::SUB, 
            s: true, 
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
        }))
    } else {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: true, 
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
        }))
    }
}

fn decode_add_subtract_immediate<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, sub, immed, rn, rd)): ((&[u8], usize), (u8, bool, u16, u8, u8)) = 
    tuple(
        (
            tag(0b000111, 6usize),
            bool,
            take(3usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    if sub {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::SUB, 
            s: true, 
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { immed: immed, rotate: 0 }
        }))
    } else {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: true, 
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { immed: immed, rotate: 0 }
        }))
    }
}

fn decode_add_subtract_compare_move_immediate<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opcode, reg, immed)): ((&[u8], usize), (u8, u8, u8, u16)) = 
    tuple(
        (
            tag(0b001, 3usize),
            take(2usize),
            take(3usize),
            take(8usize)
        )
    )(inst)?;

    match opcode  {
        0b00 => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::MOV, 
            s: true, 
            rn: Register::try_from(reg).unwrap(), 
            rd: Register::try_from(reg).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),
        0b01 => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::CMP, 
            s: true, 
            rn: Register::try_from(reg).unwrap(), 
            rd: Register::try_from(reg).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),
        0b10 => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: true, 
            rn: Register::try_from(reg).unwrap(), 
            rd: Register::try_from(reg).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),
        _ => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::SUB, 
            s: true, 
            rn: Register::try_from(reg).unwrap(), 
            rd: Register::try_from(reg).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),        
    }
}

fn decode_data_processing_register<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opcode, rm_rs, rd_rn)): ((&[u8], usize), (u8, u8, u8, u8)) =
    tuple(
        (
            tag(0b010000, 6usize),
            take(4usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    let mut op: DataProcessingOpcode = DataProcessingOpcode::try_from(opcode).unwrap();
    if op == DataProcessingOpcode::MOV { op = DataProcessingOpcode::MUL; }

    let (op, shifter_operand) = match opcode {
        0b0000 => ( // AND
            DataProcessingOpcode::AND, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b0001 => ( // EOR
            DataProcessingOpcode::EOR, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b0010 => ( // LSL
            DataProcessingOpcode::MOV, 
            DPShifterOperand::RegisterShift { 
                rs: Register::try_from(rm_rs).unwrap(), 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rd_rn).unwrap() 
            }),
        0b0011 => ( // LSR
            DataProcessingOpcode::MOV, 
            DPShifterOperand::RegisterShift { 
                rs: Register::try_from(rm_rs).unwrap(), 
                shift_type: ShiftType::LSR, 
                rm: Register::try_from(rd_rn).unwrap() 
            }),
        0b0100 => ( // ASR
            DataProcessingOpcode::MOV, 
            DPShifterOperand::RegisterShift { 
                rs: Register::try_from(rm_rs).unwrap(), 
                shift_type: ShiftType::ASR, 
                rm: Register::try_from(rd_rn).unwrap() 
            }),
        0b0101 => ( // ADC
            DataProcessingOpcode::ADC, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b0110 => ( // SBC
            DataProcessingOpcode::SBC, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b0111 => ( // ROR
            DataProcessingOpcode::MOV, 
            DPShifterOperand::RegisterShift { 
                rs: Register::try_from(rm_rs).unwrap(), 
                shift_type: ShiftType::ROR, 
                rm: Register::try_from(rd_rn).unwrap() 
            }),
        0b1000 => ( // TST
            DataProcessingOpcode::TST, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1001 => ( // NEG
            DataProcessingOpcode::RSB, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1010 => ( // CMP
            DataProcessingOpcode::CMP, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1011 => ( // CMN
            DataProcessingOpcode::CMN, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1100 => ( // ORR
            DataProcessingOpcode::ORR, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1101 => ( // MUL
            DataProcessingOpcode::MUL, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1110 => ( // BIC
            DataProcessingOpcode::BIC, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        0b1111 => ( // MVN
            DataProcessingOpcode::MVN, 
            DPShifterOperand::ImmediateShift { 
                immed: 0, 
                shift_type: ShiftType::LSL, 
                rm: Register::try_from(rm_rs).unwrap() 
            }),
        _ => unreachable!(),
    };

    Ok((i, ArmInstruction::DataProcessing { 
        c: Condition::AL, 
        op, 
        s: true, 
        rn: Register::try_from(rd_rn).unwrap(), 
        rd: Register::try_from(rd_rn).unwrap(), 
        shifter_operand
    }))
}

fn decode_special_data_processing<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opcode, h1, h2, rm, rd_rn)): ((&[u8], usize), (u8, u8, u8, u8, u8, u8)) =
    tuple(
        (
            tag(0b010001, 6usize),
            take(2usize),
            take(1usize),
            take(1usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::DataProcessing { 
        c: Condition::AL, 
        op: match opcode {
            0b00 => DataProcessingOpcode::ADD,
            0b01 => DataProcessingOpcode::CMP,
            0b10 => DataProcessingOpcode::MOV,
            _ => DataProcessingOpcode::MOV // ??
        }, 
        s: false, 
        rn: Register::try_from((h1 << 3) | rd_rn).unwrap(), 
        rd: Register::try_from((h1 << 3) | rd_rn).unwrap(), 
        shifter_operand: DPShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from((h2 << 3) | rm).unwrap() }
    }))
}

fn decode_branch_exchange<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, l, rm, _)): ((&[u8], usize), (u8, bool, u8, u8)) =
    tuple(
        (
            tag(0b01000111, 8usize),
            bool,
            take(4usize),
            take(3usize),
        )
    )(inst)?;

    if l {
        Ok((i, ArmInstruction::Branch { 
            c: Condition::AL, 
            op: BranchOperation::BranchExchangeLinkThumb { rm: Register::try_from(rm).unwrap() }
        }))
    } else {
        Ok((i, ArmInstruction::Branch { 
            c: Condition::AL, 
            op: BranchOperation::BranchExchangeThumb { rm: Register::try_from(rm).unwrap() }
        }))
    }
}

fn decode_load_from_literal_pool<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, reg, immed)): ((&[u8], usize), (u8, u8, u8)) = 
    tuple(
        (
            tag(0b01001, 5usize),
            take(3usize),
            take(8usize)
        )
    )(inst)?;

    Ok((i, ArmInstruction::LoadStore { 
        c: Condition::AL, 
        pre_indexed: true, 
        add_offset: true, 
        width: BusWidth::Word, 
        w: false, 
        load: true, 
        rn: Register::R15, 
        rd: Register::try_from(reg).unwrap(), 
        shifter_operand: crate::decode_arm::LSShifterOperand::Immediate { immed: (immed as u16) << 2 } 
    }))
}

fn decode_load_store_register_offset<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opcode, rm, rn, rd)): ((&[u8], usize), (u8, u8, u8, u8, u8)) = 
    tuple(
        (
            tag(0b0101, 4usize),
            take(3usize),
            take(3usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    match opcode {
        0b000 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                width: BusWidth::Word, 
                w: false, 
                load: false, 
                rn: Register::try_from(rn).unwrap(), 
                rd: Register::try_from(rd).unwrap(), 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
            })),
        0b001 => 
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                width: BusWidth::HalfWord, 
                w: false, 
                load: false, 
                rn: Register::try_from(rn).unwrap(), 
                rd: Register::try_from(rd).unwrap(), 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
            })),
        0b010 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                width: BusWidth::Byte, 
                w: false, 
                load: false, 
                rn: Register::try_from(rn).unwrap(), 
                rd: Register::try_from(rd).unwrap(), 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
            })),
        0b100 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                width: BusWidth::Word, 
                w: false, 
                load: true, 
                rn: Register::try_from(rn).unwrap(), 
                rd: Register::try_from(rd).unwrap(), 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
            })),
        0b101 => 
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                width: BusWidth::HalfWord, 
                w: false, 
                load: true, 
                rn: Register::try_from(rn).unwrap(), 
                rd: Register::try_from(rd).unwrap(), 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
            })),
        0b110 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                width: BusWidth::Byte, 
                w: false, 
                load: true, 
                rn: Register::try_from(rn).unwrap(), 
                rd: Register::try_from(rd).unwrap(), 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: Register::try_from(rm).unwrap() } 
            })),
        _ => fail(inst)
    }

}

fn decode_load_store_word_byte_immediate_offset<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, b, l, offset, rn, rd)): ((&[u8], usize), (u8, bool, bool, u8, u8, u8)) =
    tuple(
        (
            tag(0b011, 3usize),
            bool,
            bool,
            take(5usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::LoadStore { 
        c: Condition::AL, 
        pre_indexed: true, 
        add_offset: true, 
        width: if b { BusWidth::Byte } else { BusWidth::Word }, 
        w: false, 
        load: l, 
        rn: Register::try_from(rn).unwrap(), 
        rd: Register::try_from(rd).unwrap(), 
        shifter_operand: crate::decode_arm::LSShifterOperand::Immediate { 
            immed: 
                if b {
                    (offset as u16)
                } else {
                    (offset as u16) * 4
                } 
        } 
    }))
}

fn decode_load_store_halfword_immediate_offset<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, l, offset, rn, rd)): ((&[u8], usize), (u8, bool, u8, u8, u8)) =
    tuple(
        (
            tag(0b1000, 4usize),
            bool,
            take(5usize),
            take(3usize),
            take(3usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::LoadStore { 
        c: Condition::AL, 
        pre_indexed: true, 
        add_offset: true, 
        width: BusWidth::HalfWord, 
        w: false, 
        load: l, 
        rn: Register::try_from(rn).unwrap(), 
        rd: Register::try_from(rd).unwrap(), 
        shifter_operand: crate::decode_arm::LSShifterOperand::Immediate { immed: (offset as u16) * 2 } 
    }))
}

fn decode_load_store_from_stack<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, l, rd, offset)): ((&[u8], usize), (u8, bool, u8, u8)) =
    tuple(
        (
            tag(0b1001, 4usize),
            bool,
            take(3usize),
            take(8usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::LoadStore { 
        c: Condition::AL, 
        pre_indexed: true, 
        add_offset: true, 
        width: BusWidth::Word, 
        w: false, 
        load: l, 
        rn: Register::R13, 
        rd: Register::try_from(rd).unwrap(), 
        shifter_operand: crate::decode_arm::LSShifterOperand::Immediate { immed: (offset as u16) * 4 } 
    }))
}

fn decode_add_to_sp_or_pc<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, sp, rd, immediate)): ((&[u8], usize), (u8, bool, u8, u8)) =
    tuple(
        (
            tag(0b1010, 4usize),
            bool,
            take(3usize),
            take(8usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::DataProcessing { 
        c: Condition::AL, 
        op: DataProcessingOpcode::ADD, 
        s: false, 
        rn: if sp { Register::R13 } else { Register::R15 }, 
        rd: Register::try_from(rd).unwrap(), 
        shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immediate as u16 * 4 } 
    }))
}

fn decode_adjust_stack_pointer<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opc, immediate)): ((&[u8], usize), (u8, bool, u8)) =
    tuple(
        (
            tag(0b10110000, 8usize),
            bool,
            take(7usize),
        )
    )(inst)?;

    if opc {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::SUB, 
            s: false, 
            rn: Register::R13, 
            rd: Register::R13, 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: (immediate as u16) << 2 }
        }))
    } else {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: false, 
            rn: Register::R13, 
            rd: Register::R13, 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: (immediate as u16) << 2 }
        }))
    }
}

fn decode_push_pop<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, l, _, r, register_list)): ((&[u8], usize), (u8, bool, u8, bool, u8)) =
    tuple(
        (
            tag(0b1011, 4usize),
            bool,
            tag(0b10, 2usize),
            bool,
            take(8usize),
        )
    )(inst)?;

    let mut register_list = RegisterList::from_bits(register_list as u16).unwrap();

    if r {
        if l {
            register_list.set(RegisterList::FLAG_R15, true);
        } else {
            register_list.set(RegisterList::FLAG_R14, true);
        }
    }

    Ok((i, ArmInstruction::LoadStoreMultiple { 
        c: Condition::AL, 
        exclude_first_word: !l, 
        upwards: l, 
        update_base: true, 
        load_usermode: false, 
        load: l, 
        rn: Register::R13, 
        register_list,
    }))

    /*if l {
        Ok((i, ArmInstruction::Pop { 
            c: Condition::AL, 
            r,
            register_list: RegisterList::from_bits(register_list as u16).unwrap()
        })) 
    } else {
        Ok((i, ArmInstruction::Push { 
            c: Condition::AL, 
            r,
            register_list: RegisterList::from_bits(register_list as u16).unwrap()
        })) 
    }*/
}

fn decode_load_store_multiple<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, l, rn, register_list)): ((&[u8], usize), (u8, bool, u8, u8)) =
    tuple(
        (
            tag(0b1100, 4usize),
            bool,
            take(3usize),
            take(8usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::LoadStoreMultiple { 
        c: Condition::AL, 
        exclude_first_word: false,
        load_usermode: false,
        update_base: true,
        upwards: true,
        load: l, 
        rn: Register::try_from(rn).unwrap(), 
        register_list: RegisterList::from_bits(register_list as u16).unwrap()
    })) 
}

fn decode_branch_conditional<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, cond, offset)): ((&[u8], usize), (u8, u8, u8)) =
    tuple(
        (
            tag(0b1101, 4usize),
            take(4usize),
            take(8usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::Branch { 
        c: Condition::try_from(cond).unwrap(), 
        op: BranchOperation::BranchImmed { offset: sign_extend((offset as u32).shl(1), 9) }
    }))
}

fn decode_unconditional_branch<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, offset)): ((&[u8], usize), (u8, u16)) =
    tuple(
        (
            tag(0b11100, 5usize),
            take(11usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::Branch { 
        c: Condition::AL, 
        op: BranchOperation::BranchImmed { offset: sign_extend((offset as u32).shl(1), 12) as i32 }
    }))
}

fn decode_invalid<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    fail(inst)
}

fn decode_bl_blx_prefix<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, offset_hi)): ((&[u8], usize), (u8, u16)) =
    tuple(
        (
            tag(0b11110, 5usize),
            take(11usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::BranchLinkPrefix { offset: offset_hi }))
}

fn decode_blx_suffix<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, offset_lo)): ((&[u8], usize), (u8, u16)) =
    tuple(
        (
            tag(0b11101, 5usize),
            take(11usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::BranchLinkSuffix { op: BranchOperation::BranchExchangeLinkThumbImmed { offset: offset_lo as i32 } }))
}

fn decode_bl_suffix<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, offset_lo)): ((&[u8], usize), (u8, u16)) =
    tuple(
        (
            tag(0b11111, 5usize),
            take(11usize),
        )
    )(inst)?;

    Ok((i, ArmInstruction::BranchLinkSuffix { op: BranchOperation::BranchLinkImmed { offset: offset_lo as i32, lr_correct: 0 }}))
}

pub fn generate_thumb_instruction_table() -> Vec<ThumbInstructionTableEntry> {
    let mut t = INSTRUCTION_TABLE.to_vec();
    t.sort_by(|a,b| b.0.count_ones().cmp(&a.0.count_ones())); 
    t
}

impl GbaSystem {
    pub fn decode_thumb_instruction(&self, inst: u16) -> Option<ArmInstruction> {
        // Go to instruction table and call the references parser function if a match is found
        for i in &self.thumb_instruction_table {
            if inst & i.0 == i.1 {
                let b = inst.to_be_bytes();
                let res: IResult<&[u8], ArmInstruction> = bits(i.2)(b.as_ref());
                if let Ok(inst) = res {
                    return Some(inst.1);
                } else {
                    return None;
                }
            }
        }

        None
    }
}