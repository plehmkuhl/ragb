use crate::bus::BusWidth;
use crate::gba::*;
use crate::alu::*;
use bitflags::bitflags;

use nom::{
    IResult,
    bits,
    bits::complete::{bool, tag, take},
    sequence::tuple, combinator::fail,
};
use std::{fmt, ops::Shr};

#[derive(Copy, Clone)]
pub enum Condition {
    EQ      = 0b0000,
    NE      = 0b0001,
    CSHS    = 0b0010,
    CCLO    = 0b0011,
    MI      = 0b0100,
    PL      = 0b0101,
    VS      = 0b0110,
    VC      = 0b0111,
    HI      = 0b1000,
    LS      = 0b1001,
    GE      = 0b1010,
    LT      = 0b1011,
    GT      = 0b1100,
    LE      = 0b1101,
    AL      = 0b1110,
    UNC     = 0b1111,
}

impl TryFrom<u8> for Condition {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == Condition::EQ as u8 => Ok(Condition::EQ),
            x if x == Condition::NE as u8 => Ok(Condition::NE),
            x if x == Condition::CSHS as u8 => Ok(Condition::CSHS),
            x if x == Condition::CCLO as u8 => Ok(Condition::CCLO),
            x if x == Condition::MI as u8 => Ok(Condition::MI),
            x if x == Condition::PL as u8 => Ok(Condition::PL),
            x if x == Condition::VS as u8 => Ok(Condition::VS),
            x if x == Condition::VC as u8 => Ok(Condition::VC),
            x if x == Condition::HI as u8 => Ok(Condition::HI),
            x if x == Condition::LS as u8 => Ok(Condition::LS),
            x if x == Condition::GE as u8 => Ok(Condition::GE),
            x if x == Condition::LT as u8 => Ok(Condition::LT),
            x if x == Condition::GT as u8 => Ok(Condition::GT),
            x if x == Condition::LE as u8 => Ok(Condition::LE),
            x if x == Condition::AL as u8 => Ok(Condition::AL),
            x if x == Condition::UNC as u8 => Ok(Condition::UNC),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Condition::EQ => write!(f, "EQ"),
            Condition::NE => write!(f, "NE"),
            Condition::CSHS => write!(f, "CS"),
            Condition::CCLO => write!(f, "CC"),
            Condition::MI => write!(f, "MI"),
            Condition::PL => write!(f, "PL"),
            Condition::VS => write!(f, "VS"),
            Condition::VC => write!(f, "VC"),
            Condition::HI => write!(f, "HI"),
            Condition::LS => write!(f, "LS"),
            Condition::GE => write!(f, "GE"),
            Condition::LT => write!(f, "LT"),
            Condition::GT => write!(f, "GT"),
            Condition::LE => write!(f, "LE"),
            Condition::AL => write!(f, ""),
            Condition::UNC => write!(f, ""),
        }
    }
}

#[derive(Copy, Clone)]
#[derive(PartialEq)]
pub enum DataProcessingOpcode {
    AND = 0b0000,
    EOR = 0b0001,
    SUB = 0b0010,
    RSB = 0b0011,
    ADD = 0b0100,
    ADC = 0b0101,
    SBC = 0b0110,
    RSC = 0b0111,
    TST = 0b1000,
    TEQ = 0b1001,
    CMP = 0b1010,
    CMN = 0b1011,
    ORR = 0b1100,
    MOV = 0b1101,
    BIC = 0b1110,
    MVN = 0b1111,
    MUL,
}

impl TryFrom<u8> for DataProcessingOpcode {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == DataProcessingOpcode::AND as u8 => Ok(DataProcessingOpcode::AND),
            x if x == DataProcessingOpcode::EOR as u8 => Ok(DataProcessingOpcode::EOR),
            x if x == DataProcessingOpcode::SUB as u8 => Ok(DataProcessingOpcode::SUB),
            x if x == DataProcessingOpcode::RSB as u8 => Ok(DataProcessingOpcode::RSB),
            x if x == DataProcessingOpcode::ADD as u8 => Ok(DataProcessingOpcode::ADD),
            x if x == DataProcessingOpcode::ADC as u8 => Ok(DataProcessingOpcode::ADC),
            x if x == DataProcessingOpcode::SBC as u8 => Ok(DataProcessingOpcode::SBC),
            x if x == DataProcessingOpcode::RSC as u8 => Ok(DataProcessingOpcode::RSC),
            x if x == DataProcessingOpcode::TST as u8 => Ok(DataProcessingOpcode::TST),
            x if x == DataProcessingOpcode::TEQ as u8 => Ok(DataProcessingOpcode::TEQ),
            x if x == DataProcessingOpcode::CMP as u8 => Ok(DataProcessingOpcode::CMP),
            x if x == DataProcessingOpcode::CMN as u8 => Ok(DataProcessingOpcode::CMN),
            x if x == DataProcessingOpcode::ORR as u8 => Ok(DataProcessingOpcode::ORR),
            x if x == DataProcessingOpcode::MOV as u8 => Ok(DataProcessingOpcode::MOV),
            x if x == DataProcessingOpcode::BIC as u8 => Ok(DataProcessingOpcode::BIC),
            x if x == DataProcessingOpcode::MVN as u8 => Ok(DataProcessingOpcode::MVN),
            _ => Err(()),
        }
    }
}

impl fmt::Display for DataProcessingOpcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DataProcessingOpcode::ADD => write!(f, "ADD"),
            DataProcessingOpcode::ADC => write!(f, "ADC"),
            DataProcessingOpcode::AND => write!(f, "AND"),
            DataProcessingOpcode::BIC => write!(f, "BIC"),
            DataProcessingOpcode::CMN => write!(f, "CMN"),
            DataProcessingOpcode::CMP => write!(f, "CMP"),
            DataProcessingOpcode::EOR => write!(f, "EOR"),
            DataProcessingOpcode::MOV => write!(f, "MOV"),
            DataProcessingOpcode::MVN => write!(f, "MVN"),
            DataProcessingOpcode::ORR => write!(f, "ORR"),
            DataProcessingOpcode::RSB => write!(f, "RSB"),
            DataProcessingOpcode::RSC => write!(f, "RSC"),
            DataProcessingOpcode::SUB => write!(f, "SUB"),
            DataProcessingOpcode::SBC => write!(f, "SBC"),
            DataProcessingOpcode::TST => write!(f, "TST"),
            DataProcessingOpcode::TEQ => write!(f, "TEQ"),
            DataProcessingOpcode::MUL => write!(f, "MUL"),
        }
    }
}

#[derive(Copy, Clone)]
pub enum DPShifterOperand {
    Immediate{rotate: u8, immed: u16},
    ImmediateShift{immed: u8, shift_type: ShiftType, rm: Register},
    RegisterShift{rs: Register, shift_type: ShiftType, rm: Register},
}

impl fmt::Display for DPShifterOperand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DPShifterOperand::Immediate { rotate, immed } => 
                write!(f, "{:#x}", (immed as u32).rotate_right((rotate * 2).into())),
            &DPShifterOperand::ImmediateShift { immed, shift_type, rm } => 
                write!(f, "{}, {} {}", rm, shift_type, immed),
            &DPShifterOperand::RegisterShift { rs, shift_type, rm } => 
            write!(f, "{}, {} {}", rm, shift_type, rs),
        }
    }
}

#[derive(Copy, Clone)]
pub enum LSShifterOperand {
    Immediate{immed: u16},
    ImmediateShift{immed: u8, shift_type: ShiftType, rm: Register},
}

impl fmt::Display for LSShifterOperand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &LSShifterOperand::Immediate { immed } => 
                write!(f, "#{:#x}", immed),
            &LSShifterOperand::ImmediateShift { immed, shift_type, rm } => 
                write!(f, "{}, {} #{}", rm, shift_type, immed),
        }
    }
}

#[derive(Copy, Clone)]
pub enum ShiftType {
    ASR,
    LSL,
    LSR,
    ROR,
    RRX,
}

impl fmt::Display for ShiftType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ShiftType::ASR => write!(f, "ASR"),
            &ShiftType::LSL => write!(f, "LSL"),
            &ShiftType::LSR => write!(f, "LSR"),
            &ShiftType::ROR => write!(f, "ROR"),
            &ShiftType::RRX => write!(f, "RRX"),
        }
    }
}

impl TryFrom<u8> for ShiftType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x0 => Ok(ShiftType::LSL),
            0x1 => Ok(ShiftType::LSR),
            0x2 => Ok(ShiftType::ASR),
            0x3 => Ok(ShiftType::ROR),
            _ => Err(()),
        }
    }
}

#[derive(Copy, Clone)]
pub enum LoadStoreStatusOperation {
    StatusRegisterToRegister{rd: Register},
    RegisterToStatusRegister{mask: MSRMask, rm: Register},
    ImmediateToStatusRegister{mask: MSRMask, rot_imm: u8, immed: u8},
}

impl fmt::Display for LoadStoreStatusOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoadStoreStatusOperation::StatusRegisterToRegister { rd } => write!(f, ""),
            LoadStoreStatusOperation::RegisterToStatusRegister { mask, rm } => write!(f, ""),
            LoadStoreStatusOperation::ImmediateToStatusRegister { mask, rot_imm, immed } => write!(f, ""),
        }
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct MSRMask: u8 {
        const FLAG_C = 0b0001;
        const FLAG_X = 0b0010;
        const FLAG_S = 0b0100;
        const FLAG_F = 0b1000;
    }
}

impl fmt::Display for MSRMask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}{}", 
            if self.contains(MSRMask::FLAG_C) { "c" } else { "" },
            if self.contains(MSRMask::FLAG_X) { "x" } else { "" },
            if self.contains(MSRMask::FLAG_S) { "s" } else { "" },
            if self.contains(MSRMask::FLAG_F) { "f" } else { "" },
        )
    }
}

impl MSRMask {
    pub fn get_mask(&self) -> u32 {
        (if self.contains(MSRMask::FLAG_C) { 0x000000FF } else { 0x00000000 }) |
        (if self.contains(MSRMask::FLAG_X) { 0x0000FF00 } else { 0x00000000 }) |
        (if self.contains(MSRMask::FLAG_S) { 0x00FF0000 } else { 0x00000000 }) |
        (if self.contains(MSRMask::FLAG_F) { 0xFF000000 } else { 0x00000000 })
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    #[derive(PartialEq, Eq)]
    pub struct RegisterList : u16 {
        const FLAG_R0 =  0b0000000000000001;
        const FLAG_R1 =  0b0000000000000010;
        const FLAG_R2 =  0b0000000000000100;
        const FLAG_R3 =  0b0000000000001000;
        const FLAG_R4 =  0b0000000000010000;
        const FLAG_R5 =  0b0000000000100000;
        const FLAG_R6 =  0b0000000001000000;
        const FLAG_R7 =  0b0000000010000000;
        const FLAG_R8 =  0b0000000100000000;
        const FLAG_R9 =  0b0000001000000000;
        const FLAG_R10 = 0b0000010000000000;
        const FLAG_R11 = 0b0000100000000000;
        const FLAG_R12 = 0b0001000000000000;
        const FLAG_R13 = 0b0010000000000000;
        const FLAG_R14 = 0b0100000000000000;
        const FLAG_R15 = 0b1000000000000000;
    }
}

impl RegisterList {
    pub fn count(&self) -> u32 {
        self.0.bits.count_ones()
    }
}

impl fmt::Display for RegisterList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut registers = Vec::new();
        for n in *self {
            match n {
                RegisterList::FLAG_R0 => registers.push(format!("{}", Register::R0)),
                RegisterList::FLAG_R1 => registers.push(format!("{}", Register::R1)),
                RegisterList::FLAG_R2 => registers.push(format!("{}", Register::R2)),
                RegisterList::FLAG_R3 => registers.push(format!("{}", Register::R3)),
                RegisterList::FLAG_R4 => registers.push(format!("{}", Register::R4)),
                RegisterList::FLAG_R5 => registers.push(format!("{}", Register::R5)),
                RegisterList::FLAG_R6 => registers.push(format!("{}", Register::R6)),
                RegisterList::FLAG_R7 => registers.push(format!("{}", Register::R7)),
                RegisterList::FLAG_R8 => registers.push(format!("{}", Register::R8)),
                RegisterList::FLAG_R9 => registers.push(format!("{}", Register::R9)),
                RegisterList::FLAG_R10 => registers.push(format!("{}", Register::R10)),
                RegisterList::FLAG_R11 => registers.push(format!("{}", Register::R11)),
                RegisterList::FLAG_R12 => registers.push(format!("{}", Register::R12)),
                RegisterList::FLAG_R13 => registers.push(format!("{}", Register::R13)),
                RegisterList::FLAG_R14 => registers.push(format!("{}", Register::R14)),
                RegisterList::FLAG_R15 => registers.push(format!("{}", Register::R15)),
                _ => unreachable!(),
            }
        }

        write!(f, "{}", registers.join(","))
    }
}

#[derive(Copy, Clone)]

pub enum BranchOperation {
    BranchImmed { offset: i32 },
    BranchLinkImmed { offset: i32, lr_correct: i32 },
    BranchExchangeThumb { rm: Register },
    BranchExchangeLinkThumb { rm: Register },
    BranchExchangeLinkThumbImmed {  offset: i32 },
    BranchExchangeJava { rm: Register },
}

pub enum ArmInstruction {
    DataProcessing{c: Condition, op: DataProcessingOpcode, s: bool, rn: Register, rd: Register, shifter_operand: DPShifterOperand},
    LoadStore{c: Condition, pre_indexed: bool, add_offset: bool, width: BusWidth, w: bool, load: bool, rn: Register, rd: Register, shifter_operand: LSShifterOperand},
    LoadStoreStatus{c: Condition, r: bool, op: LoadStoreStatusOperation},
    Branch{c: Condition, op: BranchOperation},
    //Push{c: Condition, r: bool, register_list: RegisterList},
    //Pop{c: Condition, r: bool, register_list: RegisterList},
    LoadStoreMultiple{c: Condition, exclude_first_word: bool, upwards: bool, update_base: bool, load_usermode: bool, load: bool, rn: Register, register_list: RegisterList},

    // Special two-part thumb branch instructions
    BranchLinkPrefix{ offset: u16 },
    BranchLinkSuffix{ op: BranchOperation },

    MiscellaneousInstruction{c: Condition},
    Multiplie{c: Condition},
    ExtraLoadStores{c: Condition},
    UndefinedInstruction{c: Condition},
    MoveImmediateToStatusRegister{c: Condition, r: bool, mask: u8, sbo: u8, rotate: u8, immediate: u8},
    MediaInstruction{c: Condition},
    
    CoprocessorLoadStore{c: Condition, p: bool, u: bool, n: bool, w: bool, l: bool, rn: u8, crd: u8, cp_num: u8, offset: u8},
    CoprocessorDataProcessing{c: Condition, op1: u8, crn: u8, crd: u8, cp_num: u8, op2: u8, crm: u8},
    CoprocessorRegisterTransfer{c: Condition, op1: u8, l: bool, crn: u8, crd: u8, cp_num: u8, op2: u8, crm: u8},
    SoftwareInterrupt{c: Condition, swi_number: u32},
    ChangeProcessorState{imod: u8, m: bool, sbz: u8, a: bool, i: bool, f: bool, mode: u8},
    SetEndianness{sbz_1: u8, e: bool, sbz_2: u8, sbz_3: u8},
    CachePreload{x: bool, u: bool, rn: u8, addr_mode: u16},
    SaveReturnState{p: bool, u: bool, w: bool, sbz_1: u8, sbz_2: bool, mode: u8},
    ReturnFromException{p: bool, u: bool, w: bool, rn: u8, sbz_1: u8, sbz_2: u8},
    BranchWithLinkAndChangeToThumb{h: bool, offset: u32},
}

impl fmt::Display for ArmInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ArmInstruction::DataProcessing { c, op, s, rn, rd, shifter_operand } => {
                match op {
                    DataProcessingOpcode::TST | 
                    DataProcessingOpcode::TEQ |
                    DataProcessingOpcode::CMP |
                    DataProcessingOpcode::CMN =>
                        write!(f, "{}{} {}, {}", op, c, rn, shifter_operand),
                    DataProcessingOpcode::MOV |
                    DataProcessingOpcode::MVN => 
                        write!(f, "{}{}{} {}, {}", op, c, if s { "S" } else { "" }, rd, shifter_operand),
                    _ => 
                        write!(f, "{}{}{} {}, {}, {}", op, c, if s { "S" } else { "" }, rd, rn, shifter_operand)
                }
            },
            ArmInstruction::LoadStore { c, pre_indexed: p, add_offset: u, width, w, load: l, rn, rd, shifter_operand } => {
                let mut t = "";
                let mut adr = String::new();
                
                if p {
                    if u {
                        adr = format!("[{} {}]", rn, shifter_operand);
                    } else {
                        adr = format!("[{} -{}]", rn, shifter_operand);
                    }
                } else {
                    if u {
                        adr = format!("[{}] {}", rn, shifter_operand);
                    } else {
                        adr = format!("[{}] -{}", rn, shifter_operand);
                    }

                    if w {
                        t = "T";
                    }
                };

                write!(f, "{}{}{}{} {}, {}", 
                    if l { "LDR" } else { "STR" },
                    c, 
                    match width {
                        BusWidth::Byte => "B",
                        BusWidth::HalfWord => "H",
                        BusWidth::Word => "",
                    }, 
                    t,
                    rd, 
                    adr
                )
            },
            ArmInstruction::LoadStoreStatus { c, r, op } => {
                match op {
                    LoadStoreStatusOperation::StatusRegisterToRegister { rd } => 
                        write!(f, "MRS{} {}, {}", c, rd, if r { "SPSR" } else { "CPSR" }),
                    LoadStoreStatusOperation::RegisterToStatusRegister { mask, rm } => 
                        write!(f, "MSR{} {}_{}, {}", c, if r { "SPSR" } else { "CPSR" }, mask, rm),
                    LoadStoreStatusOperation::ImmediateToStatusRegister { mask, rot_imm, immed } => 
                        write!(f, "MSR{} {}_{}, {:08x}", c, if r { "SPSR" } else { "CPSR" }, mask, (immed as u32).rotate_right(rot_imm as u32 * 2)),
                }
            },
            ArmInstruction::LoadStoreMultiple {c, exclude_first_word, upwards, update_base, load_usermode, load, rn, register_list} => {
                if load {
                    write!(f, "LDM{}{} {}!, {{{}}}", 
                        if exclude_first_word {
                            if upwards {
                                "IB"
                            } else {
                                "DB"
                            }
                        } else {
                            if upwards {
                                "IA"
                            } else {
                                "DA"
                            }
                        },
                        c,
                        rn,
                        register_list)
                } else {
                    write!(f, "STM{}{} {}!, {{{}}}", 
                        if exclude_first_word {
                            if upwards {
                                "IB"
                            } else {
                                "DB"
                            }
                        } else {
                            if upwards {
                                "IA"
                            } else {
                                "DA"
                            }
                        },
                        c,
                        rn,
                        register_list)
                }
            },
            ArmInstruction::Branch {c, op} => match op {
                BranchOperation::BranchImmed { offset } => write!(f, "B{} {:#x}", c, offset),
                BranchOperation::BranchLinkImmed { offset, lr_correct } => write!(f, "BL{} {:#x}", c, offset),
                BranchOperation::BranchExchangeThumb { rm} => write!(f, "BX{} {}", c, rm),
                BranchOperation::BranchExchangeLinkThumb { rm } => write!(f, "BLX{} {}", c, rm),
                BranchOperation::BranchExchangeLinkThumbImmed { offset } => write!(f, "BLX{} {:#x}", c, offset),
                BranchOperation::BranchExchangeJava { rm } => write!(f, "BJ{} {}", c, rm),
            },
            /*ArmInstruction::Push{c, r, register_list} => {
                 write!(f, "PUSH {{{}{}}}", register_list, if r { ",lr" } else {""})
            },
            ArmInstruction::Pop{c, r, register_list} => {
                write!(f, "POP {{{}{}}}", register_list, if r { ",pc" } else {""})
            },*/
            ArmInstruction::BranchLinkPrefix{ offset } => {
                write!(f, " > {:#x}", sign_extend((offset as u32) << 12, 22))
            },
            ArmInstruction::BranchLinkSuffix{ op } => {
                match op {
                    BranchOperation::BranchLinkImmed { offset, lr_correct } => write!(f, "BL {:#x}", offset << 1),
                    BranchOperation::BranchExchangeLinkThumbImmed { offset } => write!(f, "BLX {:#x}", offset << 1),
                    _ => write!(f, "UNKNOWN"),
                }
            },
            _ => write!(f, "UNKNOWN"),
        }
    }
}

pub type InstructionTableEntry = (u32, u32, for<'a> fn((&'a[u8], usize)) -> IResult<(&'a[u8], usize), ArmInstruction>);

const INSTRUCTION_TABLE: &'static [InstructionTableEntry] = &[
    (
        // Data processing immediate shift
        0b00001110000000000000000000010000, 
        0b00000000000000000000000000000000, 
        GbaSystem::decode_data_processing_immediate_shift
    ),
    (
        0b00001111101100000000000011110000, 
        0b00000001000000000000000000000000, 
        GbaSystem::decode_move_status_register_to_register
    ),
    (
        0b00001111101100000000000011110000, 
        0b00000001001000000000000000000000, 
        GbaSystem::decode_move_register_to_status_register
    ),
    (
        0b00001111111100000000000011110000, 
        0b00000001001000000000000000010000, 
        GbaSystem::decode_branch_exchange_thumb
    ),
    (
        0b00001111111100000000000011110000, 
        0b00000001001000000000000000010000, 
        GbaSystem::decode_branch_exchange_java
    ),
    (
        0b00001111111100000000000011110000, 
        0b00000001001000000000000000110000, 
        GbaSystem::decode_branch_link_exchange_thumb
    ),
    (
        // Data processing register shift
        0b00001110000000000000000010010000, 
        0b00000000000000000000000000010000, 
        GbaSystem::decode_data_processing_register_shift
    ),
    (
        // Move immediate to status register
        0b00001111101100000000000000000000, 
        0b00000011001000000000000000000000, 
        GbaSystem::decode_move_immed_to_status_register
    ),
    (
        // Misc
        0b00001111100100000000000010010000, 
        0b00000001000000000000000000010000, 
        GbaSystem::decode_invalid
    ),
    (
        // Multiplies / Extra load/stores
        0b00001110000000000000000010010000, 
        0b00000000000000000000000010010000, 
        GbaSystem::decode_invalid
    ),
    (
        // Data processing immediate
        0b00001110000000000000000000000000, 
        0b00000010000000000000000000000000, 
        GbaSystem::decode_data_processing_immediate
    ),
    (
        // Undefined
        0b00001111101100000000000000000000, 
        0b00000011000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Move immediate to status register
        0b00001111101100000000000000000000, 
        0b00000011001000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Load/store immediate offset
        0b00001110000000000000000000000000, 
        0b00000100000000000000000000000000, 
        GbaSystem::decode_load_store_immediate_offset
    ),
    (
        // Load/store register offset
        0b00001110000000000000000000010000, 
        0b00000110000000000000000000000000, 
        GbaSystem::decode_load_store_register_offset
    ),
    (
        // Media instructions
        0b00001110000000000000000000010000, 
        0b00000110000000000000000000010000, 
        GbaSystem::decode_invalid
    ),
    (
        // Architecturally Undefined
        0b00001111111100000000000011110000, 
        0b00000111111100000000000011110000, 
        GbaSystem::decode_invalid
    ),
    (
        // Load/store multiple
        0b00001110000000000000000000000000, 
        0b00001000000000000000000000000000, 
        GbaSystem::decode_load_store_multiple
    ),
    (
        // Branch and branch with link
        0b00001110000000000000000000000000, 
        0b00001010000000000000000000000000, 
        GbaSystem::decode_branch
    ),
    (
        // Coprocessor load/store and double registers transfers
        0b00001110000000000000000000000000, 
        0b00001100000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Coprocessor data processing
        0b00001111000000000000000000010000, 
        0b00001110000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Coprocessor register transfers
        0b00001111000000000000000000010000, 
        0b00001110000000000000000000010000, 
        GbaSystem::decode_invalid
    ),
    (
        // Software interrupt
        0b00001111000000000000000000000000, 
        0b00001111000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Change processor state
        0b11111111111100010000000000100000, 
        0b11110001000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Set endianess
        0b11111111111111110000000011110000, 
        0b11110001000000010000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Cache preload
        0b11111101011100001111000000000000, 
        0b11110101010100001111000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Save return state
        0b11111110010111110000111100000000, 
        0b11111000010011010000010100000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Return from exception
        0b11111110010100000000111100000000, 
        0b11111000000100000000101000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Branch with link and change to thumb
        0b11111110000000000000000000000000, 
        0b11111010000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Additional coprocessor double register transfer
        0b11111111111000000000000000000000, 
        0b11111100010000000000000000000000, 
        GbaSystem::decode_invalid
    ),
    (
        // Additional coprocessor register transfer
        0b11111111000000000000000000010000, 
        0b11111110000000000000000000010000, 
        GbaSystem::decode_invalid
    ),
    (
        // Undefined instruction
        0b11111111000000000000000000000000, 
        0b11111111000000000000000000000000, 
        GbaSystem::decode_invalid
    ),
];

pub fn generate_arm_instruction_table() -> Vec<InstructionTableEntry> {
    let mut t = INSTRUCTION_TABLE.to_vec();
    t.sort_by(|a,b| b.0.count_ones().cmp(&a.0.count_ones())); 
    t
}

impl GbaSystem {
    fn decode_data_processing_immediate_shift<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, opcode, s, rn, rd, shift_amount, shift, _, rm)): ((&[u8], usize), (u8, u8, u8, bool, u8, u8, u8, u8, u8, u8)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b000, 3usize), 
                    take(4usize), 
                    bool, 
                    take(4usize), 
                    take(4usize), 
                    take(5usize), 
                    take(2usize), 
                    tag(0b0, 1usize),
                    take(4usize),
                )
            )(inst)?;

        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::try_from(cond).unwrap(), 
            op: DataProcessingOpcode::try_from(opcode).unwrap(), 
            s,
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::ImmediateShift { 
                immed: shift_amount, 
                shift_type: if shift == 0x3 && shift_amount == 0 {
                    ShiftType::RRX
                } else {
                    ShiftType::try_from(shift).unwrap()
                },
                rm: Register::try_from(rm).unwrap()
            }
        }))
    }

    fn decode_data_processing_register_shift<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, opcode, s, rn, rd, rs, _, shift, _, rm)): ((&[u8], usize), (u8, u8, u8, bool, u8, u8, u8, u8, u8, u8, u8)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b000, 3usize), 
                    take(4usize), 
                    bool, 
                    take(4usize), 
                    take(4usize), 
                    take(4usize), 
                    tag(0b0, 1usize),
                    take(2usize), 
                    tag(0b1, 1usize),
                    take(4usize),
                )
            )(inst)?;

        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::try_from(cond).unwrap(), 
            op: DataProcessingOpcode::try_from(opcode).unwrap(), 
            s,
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::RegisterShift { 
                rs: Register::try_from(rs).unwrap(), 
                shift_type: ShiftType::try_from(shift).unwrap(), 
                rm: Register::try_from(rm).unwrap() 
            }
        }))
    }

    fn decode_data_processing_immediate<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, opcode, s, rn, rd, rotate, immed)): ((&[u8], usize), (u8, u8, u8, bool, u8, u8, u8, u16)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b001, 3usize), 
                    take(4usize), 
                    bool, 
                    take(4usize), 
                    take(4usize), 
                    take(4usize), 
                    take(8usize), 
                )
            )(inst)?;

        Ok((i, ArmInstruction::DataProcessing { 
            c: Condition::try_from(cond).unwrap(), 
            op: DataProcessingOpcode::try_from(opcode).unwrap(), 
            s,
            rn: Register::try_from(rn).unwrap(), 
            rd: Register::try_from(rd).unwrap(), 
            shifter_operand: DPShifterOperand::Immediate { 
                rotate, 
                immed 
            }
        }))
    }

    fn decode_load_store_immediate_offset<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, p, u, b, w, l, rn, rd, immed)): ((&[u8], usize), (u8, u8, bool, bool, bool, bool, bool, u8, u8, u16)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b010, 3usize), 
                    bool,
                    bool,
                    bool,
                    bool,
                    bool,
                    take(4usize),
                    take(4usize),
                    take(12usize)
                )
            )(inst)?;

        Ok((i, ArmInstruction::LoadStore {
            c: Condition::try_from(cond).unwrap(), pre_indexed: p, add_offset: u, width: if b { BusWidth::Byte } else { BusWidth::Word }, w, load: l, rn: Register::try_from(rn).unwrap(), rd: Register::try_from(rd).unwrap(),
            shifter_operand: LSShifterOperand::Immediate { immed }
        }))
    }

    fn decode_load_store_register_offset<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, p, u, b, w, l, rn, rd, shift_amount, shift, _, rm)): ((&[u8], usize), (u8, u8, bool, bool, bool, bool, bool, u8, u8, u8, u8, u8, u8)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b011, 3usize), 
                    bool,
                    bool,
                    bool,
                    bool,
                    bool,
                    take(4usize),
                    take(4usize),
                    take(5usize),
                    take(2usize),
                    tag(0b0, 1usize),
                    take(4usize),
                )
            )(inst)?;

        Ok((i, ArmInstruction::LoadStore { 
            c: Condition::try_from(cond).unwrap(), pre_indexed: p, add_offset: u, width: if b { BusWidth::Byte } else { BusWidth::Word }, w, load: l, rn: Register::try_from(rn).unwrap(), rd: Register::try_from(rd).unwrap(),
            shifter_operand: LSShifterOperand::ImmediateShift {
                immed: shift_amount, 
                shift_type: if shift_amount == 0 && shift == 0x3 {
                    ShiftType::RRX
                } else {
                    ShiftType::try_from(shift).unwrap()
                }, 
                rm: Register::try_from(rm).unwrap()
            }
        }))
    }

    fn decode_load_store_multiple<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, p, u, s, w, l, rn, register_list)): ((&[u8], usize), (u8, u8, bool, bool, bool, bool, bool, u8, u16)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b100, 3usize), 
                    bool,
                    bool,
                    bool,
                    bool,
                    bool,
                    take(4usize),
                    take(16usize),
                )
            )(inst)?;

        Ok((i, ArmInstruction::LoadStoreMultiple { 
            c: Condition::try_from(cond).unwrap(), 
            exclude_first_word: p, 
            upwards: u, 
            update_base: w, 
            load_usermode: s, 
            load: l, 
            rn: Register::try_from(rn).unwrap(), 
            register_list: RegisterList::from_bits(register_list).unwrap()
        }))
    }

    fn decode_branch<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, l, offset)): ((&[u8], usize), (u8, u8, bool, u32)) = 
            tuple(
                (
                    take(4usize), 
                    tag(0b101, 3usize), 
                    bool, 
                    take(24usize),
                )
            )(inst)?;

        // Sign extend to 32-bit and shift left by two
        let offset_adr = sign_extend(offset << 2, 26);

        if l {
            Ok((i, ArmInstruction::Branch { 
                c: Condition::try_from(cond).unwrap(), 
                op: BranchOperation::BranchLinkImmed { offset: offset_adr, lr_correct: 0 }
            }))
        } else {
            Ok((i, ArmInstruction::Branch { 
                c: Condition::try_from(cond).unwrap(), 
                op: BranchOperation::BranchImmed { offset: offset_adr }
            }))
        }

    }

    fn decode_move_status_register_to_register<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, r, _, _, rd, _, _, _)): ((&[u8], usize), (u8, u8, bool, u8, u8, u8, u8, u8, u8)) = 
        tuple(
            (
                take(4usize), 
                tag(0b00010, 5usize), 
                bool, 
                tag(0b00, 2usize),
                take(4usize),
                take(4usize),
                take(4usize),
                tag(0b0000, 4usize),
                take(4usize)
            )
        )(inst)?;

        Ok((i, ArmInstruction::LoadStoreStatus { 
            c: Condition::try_from(cond).unwrap(), 
            r, 
            op: LoadStoreStatusOperation::StatusRegisterToRegister { rd: Register::try_from(rd).unwrap() } }))
    }

    fn decode_move_register_to_status_register<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, r, _, mask, _, _, _, rm)): ((&[u8], usize), (u8, u8, bool, u8, u8, u8, u8, u8, u8)) = 
        tuple(
            (
                take(4usize), 
                tag(0b00010, 5usize), 
                bool, 
                tag(0b10, 2usize),
                take(4usize),
                take(4usize),
                take(4usize),
                tag(0b0000, 4usize),
                take(4usize)
            )
        )(inst)?;

        Ok((i, ArmInstruction::LoadStoreStatus { 
            c: Condition::try_from(cond).unwrap(), 
            r, 
            op: LoadStoreStatusOperation::RegisterToStatusRegister { mask: MSRMask::from_bits(mask).unwrap(), rm: Register::try_from(rm).unwrap() } }))
    }

    fn decode_move_immed_to_status_register<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, r, _, mask, _, rot_imm, immed)): ((&[u8], usize), (u8, u8, bool, u8, u8, u8, u8, u8)) = 
        tuple(
            (
                take(4usize), 
                tag(0b00010, 5usize), 
                bool, 
                tag(0b10, 2usize),
                take(4usize),
                take(4usize),
                take(4usize),
                take(8usize)
            )
        )(inst)?;

        Ok((i, ArmInstruction::LoadStoreStatus { 
            c: Condition::try_from(cond).unwrap(), 
            r, 
            op: LoadStoreStatusOperation::ImmediateToStatusRegister { mask: MSRMask::from_bits(mask).unwrap(), rot_imm, immed } }))
    }

    fn decode_branch_exchange_thumb<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, _, _, _, _, rm)): ((&[u8], usize), (u8, u8, u8, u8, u8, u8, u8)) = 
        tuple(
            (
                take(4usize), 
                tag(0b00010010, 8usize),
                take(4usize),
                take(4usize),
                take(4usize),
                tag(0b0001, 4usize),
                take(4usize),
            )
        )(inst)?;

        Ok((i, ArmInstruction::Branch { 
            c: Condition::try_from(cond).unwrap(), 
            op: BranchOperation::BranchExchangeThumb { rm: Register::try_from(rm).unwrap() } 
        }))
    }

    fn decode_branch_exchange_java<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, _, _, _, _, rm)): ((&[u8], usize), (u8, u8, u8, u8, u8, u8, u8)) = 
        tuple(
            (
                take(4usize), 
                tag(0b00010010, 8usize),
                take(4usize),
                take(4usize),
                take(4usize),
                tag(0b0001, 4usize),
                take(4usize),
            )
        )(inst)?;

        Ok((i, ArmInstruction::Branch { 
            c: Condition::try_from(cond).unwrap(), 
            op: BranchOperation::BranchExchangeJava { rm: Register::try_from(rm).unwrap() } 
        }))
    }

    fn decode_branch_link_exchange_thumb<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        let (i, (cond, _, _, _, _, _, rm)): ((&[u8], usize), (u8, u8, u8, u8, u8, u8, u8)) = 
        tuple(
            (
                take(4usize), 
                tag(0b00010010, 8usize),
                take(4usize),
                take(4usize),
                take(4usize),
                tag(0b0001, 4usize),
                take(4usize),
            )
        )(inst)?;

        Ok((i, ArmInstruction::Branch { 
            c: Condition::try_from(cond).unwrap(), 
            op: BranchOperation::BranchExchangeLinkThumb { rm: Register::try_from(rm).unwrap() } 
        }))
    }

    fn decode_invalid<'b>(inst: (&'b[u8], usize)) -> IResult<(&'b[u8], usize), ArmInstruction> {
        fail(inst)
    }

    pub fn decode_instruction(&self, inst: u32) -> Option<ArmInstruction> {
        // Go to instruction table and call the references parser function if a match is found
        for i in &self.instruction_table {
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
