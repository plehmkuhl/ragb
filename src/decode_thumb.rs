use crate::gba::*;
use crate::decode_arm::{ArmInstruction, Condition, DataProcessingOpcode, DPShifterOperand, ShiftType, BranchOperation};

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
        decode_invalid
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
        decode_invalid
    ),
    (
        // Special data processing
        0b1111110000000000,
        0b0100010000000000,
        decode_invalid
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
        decode_invalid
    ),
    (
        // Load/store halfword immediate offset
        0b1111000000000000,
        0b1000000000000000,
        decode_invalid
    ),
    (
        // Load/store from stack
        0b1111000000000000,
        0b1001000000000000,
        decode_invalid
    ),
    (
        // Add to SP or PC
        0b1111000000000000,
        0b1010000000000000,
        decode_invalid
    ),
    (
        // Load/store multiple
        0b1111000000000000,
        0b1100000000000000,
        decode_invalid
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
        decode_invalid
    ),
    (
        // BLX suffix
        0b1111100000000001,
        0b1110100000000000,
        decode_invalid
    ),
    (
        // BL suffix
        0b1111100000000000,
        0b1111100000000000,
        decode_invalid
    )
];

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
            rn, 
            rd, 
            shifter_operand: DPShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: rm as usize } 
        }))
    } else {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: true, 
            rn, 
            rd, 
            shifter_operand: DPShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: rm as usize } 
        }))
    }
}

fn decode_add_subtract_immediate<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, sub, immed, rn, rd)): ((&[u8], usize), (u8, bool, u8, u8, u8)) = 
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
            rn, 
            rd, 
            shifter_operand: DPShifterOperand::Immediate { immed: immed, rotate: 0 }
        }))
    } else {
        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: true, 
            rn, 
            rd, 
            shifter_operand: DPShifterOperand::Immediate { immed: immed, rotate: 0 }
        }))
    }
}

fn decode_add_subtract_compare_move_immediate<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    let (i, (_, opcode, reg, immed)): ((&[u8], usize), (u8, u8, u8, u8)) = 
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
            rn: reg, 
            rd: reg, 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),
        0b01 => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::CMP, 
            s: true, 
            rn: reg, 
            rd: reg, 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),
        0b10 => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::ADD, 
            s: true, 
            rn: reg, 
            rd: reg, 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),
        _ => Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::AL, 
            op: DataProcessingOpcode::SUB, 
            s: true, 
            rn: reg, 
            rd: reg, 
            shifter_operand: DPShifterOperand::Immediate { rotate: 0, immed: immed } })),        
    }
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
            op: BranchOperation::BranchExchangeLinkThumb { rm: rm as usize }
        }))
    } else {
        Ok((i, ArmInstruction::Branch { 
            c: Condition::AL, 
            op: BranchOperation::BranchExchangeThumb { rm: rm as usize }
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
        byte_access: false, 
        w: false, 
        load: true, 
        rn: 15, 
        rd: reg, 
        shifter_operand: crate::decode_arm::LSShifterOperand::Immediate { immed: (immed as u16) * 4 } 
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
                byte_access: false, 
                w: false, 
                load: false, 
                rn, 
                rd, 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: rm.into() } 
            })),
        0b010 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                byte_access: true, 
                w: false, 
                load: false, 
                rn, 
                rd, 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: rm.into() } 
            })),
        0b100 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                byte_access: false, 
                w: false, 
                load: true, 
                rn, 
                rd, 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: rm.into() } 
            })),
        0b110 =>     
            Ok((i, ArmInstruction::LoadStore { 
                c: Condition::AL, 
                pre_indexed: true, 
                add_offset: true, 
                byte_access: true, 
                w: false, 
                load: true, 
                rn, 
                rd, 
                shifter_operand: crate::decode_arm::LSShifterOperand::ImmediateShift { immed: 0, shift_type: ShiftType::LSL, rm: rm.into() } 
            })),
        _ => fail(inst)
    }

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
        op: BranchOperation::BranchImmed { offset: ((offset as i8) as i32).shl(1) - 2 }
    }))
}

fn decode_invalid<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
    fail(inst)
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
                return Some(res.unwrap().1);
            }
        }

        None
    }
}