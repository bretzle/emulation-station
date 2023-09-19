use std::mem::{size_of, transmute};

use crate::arm::state::{Condition, GPR};

const fn bit<const N: usize>(x: u32) -> bool {
    ((x >> N) & 1) != 0
}

const fn get_field<const START: usize, const SIZE: usize>(val: u32) -> u32 {
    (val >> START) & !(u32::MAX << SIZE)
}

const fn sign_extend<const N: usize>(val: u32) -> u32 {
    let shift = 8 * size_of::<u32>() - N;
    ((val as i32) << shift) as u32 >> shift
}

#[repr(u8)]
pub enum ShiftType {
    LSL = 0,
    LSR = 1,
    ASR = 2,
    ROR = 3,
}

impl From<u32> for ShiftType {
    fn from(value: u32) -> Self {
        unsafe { transmute(value as u8) }
    }
}

pub struct ArmSingleDataTransfer {
    pub load: bool,
    pub writeback: bool,
    pub byte: bool,
    pub up: bool,
    pub pre: bool,
    pub rd: GPR,
    pub rn: GPR,
    pub condition: Condition,
    pub rhs: ArmSingleDataTransferRhs,
}

pub enum ArmSingleDataTransferRhs {
    Imm(u32),
    Reg {
        rm: GPR,
        shift_type: ShiftType,
        amount: u32,
    },
}

impl ArmSingleDataTransfer {
    pub fn decode(inst: u32) -> Self {
        let load = bit::<20>(inst);
        let writeback = bit::<21>(inst);
        let byte = bit::<22>(inst);
        let up = bit::<23>(inst);
        let pre = bit::<24>(inst);
        let imm = !bit::<25>(inst);
        let rd = get_field::<12, 4>(inst).into();
        let rn = get_field::<16, 4>(inst).into();
        let condition = get_field::<28, 4>(inst).into();

        let rhs = if imm {
            ArmSingleDataTransferRhs::Imm(get_field::<0, 12>(inst))
        } else {
            ArmSingleDataTransferRhs::Reg {
                rm: get_field::<0, 4>(inst).into(),
                shift_type: get_field::<5, 2>(inst).into(),
                amount: get_field::<7, 5>(inst),
            }
        };

        Self {
            load,
            writeback,
            byte,
            up,
            pre,
            rd,
            rn,
            condition,
            rhs,
        }
    }
}

pub struct ArmCoprocessorRegisterTransfer {
    pub crm: GPR,
    pub crn: GPR,
    pub cp: u8,
    pub rd: GPR,
    pub load: bool,
}

impl ArmCoprocessorRegisterTransfer {
    pub fn decode(instruction: u32) -> Self {
        Self {
            crm: get_field::<0, 4>(instruction).into(),
            crn: get_field::<16, 4>(instruction).into(),
            cp: get_field::<5, 3>(instruction) as _,
            rd: get_field::<12, 4>(instruction).into(),
            load: bit::<20>(instruction),
        }
    }
}

#[repr(u8)]
pub enum Opcode {
    AND = 0,
    EOR = 1,
    SUB = 2,
    RSB = 3,
    ADD = 4,
    ADC = 5,
    SBC = 6,
    RSC = 7,
    TST = 8,
    TEQ = 9,
    CMP = 10,
    CMN = 11,
    ORR = 12,
    MOV = 13,
    BIC = 14,
    MVN = 15,
}

impl From<u32> for Opcode {
    fn from(value: u32) -> Self {
        unsafe { transmute(value as u8) }
    }
}

pub struct ArmDataProcessing {
    pub set_flags: bool,
    pub rd: GPR,
    pub rn: GPR,
    pub opcode: Opcode,
    pub condition: Condition,
    pub rhs: ArmDataProcessingRhs,
}

pub enum ArmDataProcessingRhs {
    Imm {
        shift: u32,
        rotated: u32,
    },
    Reg {
        rm: GPR,
        shift_type: ShiftType,
        amount: ArmDataProcessingAmount,
    },
}

pub enum ArmDataProcessingAmount {
    Rs(GPR),
    Imm(u8),
}

impl ArmDataProcessing {
    pub fn decode(instruction: u32) -> Self {
        let set_flags = bit::<20>(instruction);
        let imm = bit::<25>(instruction);
        let rd = get_field::<12, 4>(instruction).into();
        let rn = get_field::<16, 4>(instruction).into();
        let opcode: Opcode = get_field::<21, 4>(instruction).into();
        let condition: Condition = get_field::<28, 4>(instruction).into();

        let rhs = if imm {
            let shift = get_field::<8, 4>(instruction) * 2;
            let rotated = (instruction & 0xff).rotate_right(shift);
            ArmDataProcessingRhs::Imm { shift, rotated }
        } else {
            let rm = get_field::<0, 4>(instruction).into();
            let shift_type = get_field::<5, 2>(instruction).into();
            let imm = !bit::<4>(instruction);

            let amount = if imm {
                ArmDataProcessingAmount::Imm(get_field::<7, 5>(instruction) as _)
            } else {
                ArmDataProcessingAmount::Rs(get_field::<8, 4>(instruction).into())
            };

            ArmDataProcessingRhs::Reg {
                rm,
                shift_type,
                amount,
            }
        };

        Self {
            set_flags,
            rd,
            rn,
            opcode,
            condition,
            rhs,
        }
    }
}

pub struct ArmBranchLink {
    pub link: bool,
    pub offset: u32,
    pub condition: Condition,
}

impl ArmBranchLink {
    pub fn decode(instruction: u32) -> Self {
        Self {
            link: bit::<24>(instruction),
            offset: sign_extend::<24>(get_field::<0, 24>(instruction)) << 2,
            condition: get_field::<28, 4>(instruction).into(),
        }
    }
}

pub struct ArmBlockDataTransfer {
    pub rlist: u16,
    pub r15_in_rlist: bool,
    pub load: bool,
    pub writeback: bool,
    pub psr: bool,
    pub up: bool,
    pub pre: bool,
    pub rn: GPR,
}

impl ArmBlockDataTransfer {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rlist: get_field::<0, 16>(instruction) as _,
            r15_in_rlist: bit::<15>(instruction),
            load: bit::<20>(instruction),
            writeback: bit::<21>(instruction),
            psr: bit::<22>(instruction),
            up: bit::<23>(instruction),
            pre: bit::<24>(instruction),
            rn: get_field::<16, 4>(instruction).into(),
        }
    }
}
