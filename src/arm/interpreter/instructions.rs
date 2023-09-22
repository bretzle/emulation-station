use std::mem::transmute;

use crate::arm::state::{Condition, GPR};

const fn bit<const N: usize>(x: u32) -> bool {
    ((x >> N) & 1) != 0
}

const fn get_field<const START: usize, const SIZE: usize>(val: u32) -> u32 {
    (val >> START) & !(u32::MAX << SIZE)
}

pub const fn sign_extend<const N: usize>(val: u32) -> u32 {
    let shift = (32 - N) as u32;
    (val as i32).wrapping_shl(shift).wrapping_shr(shift) as u32
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
            offset: if instruction & (1 << 23) != 0 {
                0xFC000000
            } else {
                0
            } | (get_field::<0, 24>(instruction) << 2),
            condition: get_field::<28, 4>(instruction).into(),
        }
    }
}

pub struct ArmBlockDataTransfer {
    pub rlist: u32,
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

pub enum ArmHalfwordDataTransferRhs {
    Imm(u32),
    Reg(GPR),
}

pub struct ArmHalfwordDataTransfer {
    pub load: bool,
    pub writeback: bool,
    pub up: bool,
    pub pre: bool,
    pub half: bool,
    pub sign: bool,
    pub rd: GPR,
    pub rn: GPR,
    pub rhs: ArmHalfwordDataTransferRhs,
}

impl ArmHalfwordDataTransfer {
    pub fn decode(instruction: u32) -> Self {
        let load = bit::<20>(instruction);
        let writeback = bit::<21>(instruction);
        let imm = bit::<22>(instruction);
        let up = bit::<23>(instruction);
        let pre = bit::<24>(instruction);
        let half = bit::<5>(instruction);
        let sign = bit::<6>(instruction);
        let rd = get_field::<12, 4>(instruction).into();
        let rn = get_field::<16, 4>(instruction).into();

        let rhs = if imm {
            ArmHalfwordDataTransferRhs::Imm(((instruction >> 4) & 0xf0) | (instruction & 0xf))
        } else {
            ArmHalfwordDataTransferRhs::Reg(get_field::<0, 4>(instruction).into())
        };

        Self {
            load,
            writeback,
            up,
            pre,
            half,
            sign,
            rd,
            rn,
            rhs,
        }
    }
}

pub enum ArmStatusStoreRhs {
    Imm(u32),
    Reg(GPR),
}

pub struct ArmStatusStore {
    pub spsr: bool,
    pub mask: u32,
    pub rhs: ArmStatusStoreRhs,
}

impl ArmStatusStore {
    pub fn decode(instruction: u32) -> Self {
        let spsr = bit::<22>(instruction);
        let imm = bit::<25>(instruction);

        let mut mask = 0;
        if bit::<16>(instruction) {
            mask |= 0x000000ff;
        }
        if bit::<17>(instruction) {
            mask |= 0x0000ff00;
        }
        if bit::<18>(instruction) {
            mask |= 0x00ff0000;
        }
        if bit::<19>(instruction) {
            mask |= 0xff000000;
        }

        let rhs = if imm {
            let amount = get_field::<8, 4>(instruction) << 1;
            ArmStatusStoreRhs::Imm((instruction & 0xff).rotate_right(amount))
        } else {
            ArmStatusStoreRhs::Reg(get_field::<0, 4>(instruction).into())
        };

        Self { spsr, mask, rhs }
    }
}

pub struct ArmBranchExchange {
    pub rm: GPR,
}

impl ArmBranchExchange {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
        }
    }
}

pub struct ArmStatusLoad {
    pub spsr: bool,
    pub rd: GPR,
}

impl ArmStatusLoad {
    pub fn decode(instruction: u32) -> Self {
        let spsr = bit::<22>(instruction);
        let rd = get_field::<12, 4>(instruction).into();
        Self { spsr, rd }
    }
}

pub struct ArmMultiply {
    pub set_flags: bool,
    pub accumulate: bool,
    pub rm: GPR,
    pub rs: GPR,
    pub rn: GPR,
    pub rd: GPR,
}

impl ArmMultiply {
    pub fn decode(instruction: u32) -> Self {
        Self {
            set_flags: bit::<20>(instruction),
            accumulate: bit::<21>(instruction),
            rm: get_field::<0, 4>(instruction).into(),
            rs: get_field::<8, 4>(instruction).into(),
            rn: get_field::<12, 4>(instruction).into(),
            rd: get_field::<16, 4>(instruction).into(),
        }
    }
}

pub struct ArmMultiplyLong {
    pub set_flags: bool,
    pub accumulate: bool,
    pub sign: bool,
    pub rm: GPR,
    pub rs: GPR,
    pub rdlo: GPR,
    pub rdhi: GPR,
}

impl ArmMultiplyLong {
    pub fn decode(instruction: u32) -> Self {
        Self {
            set_flags: bit::<20>(instruction),
            accumulate: bit::<21>(instruction),
            sign: bit::<22>(instruction),
            rm: get_field::<0, 4>(instruction).into(),
            rs: get_field::<8, 4>(instruction).into(),
            rdlo: get_field::<12, 4>(instruction).into(),
            rdhi: get_field::<16, 4>(instruction).into(),
        }
    }
}

pub struct ArmSingleDataSwap {
    pub rm: GPR,
    pub rd: GPR,
    pub rn: GPR,
    pub byte: bool,
}

impl ArmSingleDataSwap {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
            rd: get_field::<12, 4>(instruction).into(),
            rn: get_field::<16, 4>(instruction).into(),
            byte: bit::<22>(instruction),
        }
    }
}

pub struct ArmCountLeadingZeros {
    pub rm: GPR,
    pub rd: GPR,
}

impl ArmCountLeadingZeros {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
            rd: get_field::<12, 4>(instruction).into(),
        }
    }
}

pub struct ArmSaturatingAddSubtract {
    pub rm: GPR,
    pub rd: GPR,
    pub rn: GPR,
    pub double_rhs: bool,
    pub sub: bool,
}

impl ArmSaturatingAddSubtract {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
            rd: get_field::<12, 4>(instruction).into(),
            rn: get_field::<16, 4>(instruction).into(),
            double_rhs: bit::<21>(instruction),
            sub: bit::<22>(instruction),
        }
    }
}

pub struct ArmSignedMultiply {
    pub rm: GPR,
    pub rs: GPR,
    pub rn: GPR,
    pub rd: GPR,
    pub accumulate: bool,
    pub x: bool,
    pub y: bool,
}

impl ArmSignedMultiply {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
            rs: get_field::<8, 4>(instruction).into(),
            rn: get_field::<12, 4>(instruction).into(),
            rd: get_field::<16, 4>(instruction).into(),
            accumulate: get_field::<21, 3>(instruction) == 0,
            x: bit::<5>(instruction),
            y: bit::<6>(instruction),
        }
    }
}

pub struct ArmBranchLinkExchange {
    pub offset: u32,
}

impl ArmBranchLinkExchange {
    pub fn decode(instruction: u32) -> Self {
        Self {
            offset: (sign_extend::<24>(get_field::<0, 24>(instruction)) << 2)
                | ((bit::<24>(instruction) as u32) << 1),
        }
    }
}

pub struct ArmSignedMultiplyWord {
    pub rm: GPR,
    pub rs: GPR,
    pub rn: GPR,
    pub rd: GPR,
    pub accumulate: bool,
    pub y: bool,
}

impl ArmSignedMultiplyWord {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
            rs: get_field::<8, 4>(instruction).into(),
            rn: get_field::<12, 4>(instruction).into(),
            rd: get_field::<16, 4>(instruction).into(),
            accumulate: !bit::<5>(instruction),
            y: bit::<6>(instruction),
        }
    }
}

pub struct ArmSignedMultiplyAccumulateLong {
    pub rm: GPR,
    pub rs: GPR,
    pub rn: GPR,
    pub rd: GPR,
    pub x: bool,
    pub y: bool,
}

impl ArmSignedMultiplyAccumulateLong {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<0, 4>(instruction).into(),
            rs: get_field::<8, 4>(instruction).into(),
            rn: get_field::<12, 4>(instruction).into(),
            rd: get_field::<16, 4>(instruction).into(),
            x: bit::<5>(instruction),
            y: bit::<6>(instruction),
        }
    }
}

pub struct ThumbAddSubtract {
    pub rd: GPR,
    pub rs: GPR,
    pub rn: GPR,
    pub sub: bool,
    pub imm: bool,
}

impl ThumbAddSubtract {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rd: get_field::<0, 3>(instruction).into(),
            rs: get_field::<3, 3>(instruction).into(),
            rn: get_field::<6, 3>(instruction).into(),
            sub: bit::<9>(instruction),
            imm: bit::<10>(instruction),
        }
    }
}

pub struct ThumbShiftImmediate {
    pub rd: GPR,
    pub rs: GPR,
    pub amount: u32,
    pub shift_type: ShiftType,
}

impl ThumbShiftImmediate {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rd: get_field::<0, 3>(instruction).into(),
            rs: get_field::<3, 3>(instruction).into(),
            amount: get_field::<6, 5>(instruction),
            shift_type: get_field::<11, 2>(instruction).into(),
        }
    }
}

pub enum ThumbALUImmediateOp {
    MOV = 0,
    CMP = 1,
    ADD = 2,
    SUB = 3,
}

struct ThumbALUImmediate {
    pub imm: u8,
    pub rd: GPR,
    pub opcode: ThumbALUImmediateOp,
}

impl ThumbALUImmediate {
    pub fn decode(instruction: u32) -> Self {
        Self {
            imm: get_field::<0, 8>(instruction) as _,
            rd: get_field::<8, 3>(instruction).into(),
            opcode: get_field::<11, 2>(instruction).into(),
        }
    }
}

pub enum ThumbOpcode {
    AND = 0,
    EOR = 1,
    LSL = 2,
    LSR = 3,
    ASR = 4,
    ADC = 5,
    SBC = 6,
    ROR = 7,
    TST = 8,
    NEG = 9,
    CMP = 10,
    CMN = 11,
    ORR = 12,
    MUL = 13,
    BIC = 14,
    MVN = 15,
}

pub struct ThumbDataProcessingRegister {
    pub rd: GPR,
    pub rs: GPR,
    pub opcode: ThumbOpcode,
}

impl ThumbDataProcessingRegister {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rd: get_field::<0, 3>(instruction).into(),
            rs: get_field::<3, 3>(instruction).into(),
            opcode: get_field::<6, 4>(instruction).into(),
        }
    }
}

pub enum SpecialOpcode {
    ADD = 0,
    CMP = 1,
    MOV = 2,
}

pub struct ThumbSpecialDataProcessing {
    pub rd: GPR,
    pub rs: GPR,
    pub opcode: SpecialOpcode,
}

impl ThumbSpecialDataProcessing {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rd: (((bit::<7>(instruction) as u32) << 3) | (get_field::<0, 3>(instruction))).into(),
            rs: get_field::<3, 4>(instruction).into(),
            opcode: get_field::<8, 2>(instruction).into(),
        }
    }
}

pub struct ThumbAdjustStackPointer {
    pub imm: u32,
    pub sub: bool,
}

impl ThumbAdjustStackPointer {
    pub fn decode(instruction: u32) -> Self {
        Self {
            imm: get_field::<0, 7>(instruction) << 2,
            sub: bit::<7>(instruction),
        }
    }
}

pub struct ThumbAddSPPC {
    pub imm: u32,
    pub rd: GPR,
    pub sp: bool,
}

impl ThumbAddSPPC {
    pub fn decode(instruction: u32) -> Self {
        Self {
            imm: get_field::<0, 8>(instruction) << 2,
            rd: get_field::<8, 3>(instruction).into(),
            sp: bit::<11>(instruction),
        }
    }
}

pub struct ThumbBranchExchange {
    pub rm: GPR,
}

impl ThumbBranchExchange {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<3, 4>(instruction).into(),
        }
    }
}

pub struct ThumbBranchLinkExchange {
    pub rm: GPR,
}

impl ThumbBranchLinkExchange {
    pub fn decode(instruction: u32) -> Self {
        Self {
            rm: get_field::<3, 4>(instruction).into(),
        }
    }
}

pub struct ThumbBranchLinkSetup {
    pub imm: u32,
}

impl ThumbBranchLinkSetup {
    pub fn decode(instruction: u32) -> Self {
        Self {
            imm: sign_extend::<11>(get_field::<0, 11>(instruction)) << 12,
        }
    }
}

pub struct ThumbBranchLinkOffset {
    pub offset: u32,
}

impl ThumbBranchLinkOffset {
    pub fn decode(instruction: u32) -> Self {
        Self {
            offset: get_field::<0, 11>(instruction) << 1,
        }
    }
}

pub struct ThumbBranchLinkExchangeOffset {
    pub offset: u32,
}

impl ThumbBranchLinkExchangeOffset {
    pub fn decode(instruction: u32) -> Self {
        Self {
            offset: get_field::<0, 11>(instruction) << 1,
        }
    }
}

pub struct ThumbBranch {
    pub offset: u32,
}

impl ThumbBranch {
    pub fn decode(instruction: u32) -> Self {
        Self {
            offset: sign_extend::<11>(get_field::<0, 11>(instruction)) << 1,
        }
    }
}

pub struct ThumbBranchConditional {
    // static ThumbBranchConditional decode(u32 instruction) {
    // ThumbBranchConditional opcode;
    // opcode.condition = static_cast<Condition>(get_field::<8, 4>(instruction));
    // opcode.offset = static_cast<u32>(common::sign_extend<s32, 8>(get_field::<0, 8>(instruction))) << 1;
    // return opcode;
    // }
    pub condition: Condition,
    pub offset: u32,
}

impl ThumbBranchConditional {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub struct ThumbLoadPC {
    // static ThumbLoadPC decode(u32 instruction) {
    // ThumbLoadPC opcode;
    // opcode.imm = get_field::<0, 8>(instruction) << 2;
    // opcode.rd = static_cast<GPR>(get_field::<8, 3>(instruction));
    // return opcode;
    // }
    pub imm: u32,
    pub rd: GPR,
}

impl ThumbLoadPC {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub enum LoadStoreRegisterOpcode {
    STR = 0,
    STRB = 1,
    LDR = 2,
    LDRB = 3,
}

pub struct ThumbLoadStoreRegisterOffset {
    // static ThumbLoadStoreRegisterOffset decode(u32 instruction) {
    // ThumbLoadStoreRegisterOffset opcode;
    // opcode.rd = static_cast<GPR>(get_field::<0, 3>(instruction));
    // opcode.rn = static_cast<GPR>(get_field::<3, 3>(instruction));
    // opcode.rm = static_cast<GPR>(get_field::<6, 3>(instruction));
    // opcode.opcode = static_cast<Opcode>(get_field::<10, 2>(instruction));
    // return opcode;
    // }
    pub rd: GPR,
    pub rn: GPR,
    pub rm: GPR,
    pub opcode: LoadStoreRegisterOpcode,
}

impl ThumbLoadStoreRegisterOffset {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub enum LoadStoreSignedOpcode {
    STRH = 0,
    LDRSB = 1,
    LDRH = 2,
    LDRSH = 3,
}

pub struct ThumbLoadStoreSigned {
    // static ThumbLoadStoreSigned decode(u32 instruction) {
    // ThumbLoadStoreSigned opcode;
    // opcode.rd = static_cast<GPR>(get_field::<0, 3>(instruction));
    // opcode.rn = static_cast<GPR>(get_field::<3, 3>(instruction));
    // opcode.rm = static_cast<GPR>(get_field::<6, 3>(instruction));
    // opcode.opcode = static_cast<Opcode>(get_field::<10, 2>(instruction));
    // return opcode;
    // }
    pub rd: GPR,
    pub rn: GPR,
    pub rm: GPR,
    pub opcode: LoadStoreSignedOpcode,
}

impl ThumbLoadStoreSigned {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub enum LoadStoreOpcode {
    STR = 0,
    LDR = 1,
    STRB = 2,
    LDRB = 3,
}

pub struct ThumbLoadStoreImmediate {
    // static ThumbLoadStoreImmediate decode(u32 instruction) {
    // ThumbLoadStoreImmediate opcode;
    // opcode.rd = static_cast<GPR>(get_field::<0, 3>(instruction));
    // opcode.rn = static_cast<GPR>(get_field::<3, 3>(instruction));
    // opcode.imm = get_field::<6, 5>(instruction);
    // opcode.opcode = static_cast<Opcode>(get_field::<11, 2>(instruction));
    // return opcode;
    // }
    pub rd: GPR,
    pub rn: GPR,
    pub imm: u32,
    pub opcode: LoadStoreOpcode,
}

impl ThumbLoadStoreImmediate {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub struct ThumbPushPop {
    // static ThumbPushPop decode(u32 instruction) {
    // ThumbPushPop opcode;
    // opcode.rlist = get_field::<0, 8>(instruction);
    // opcode.pclr = bit::<8>(instruction);
    // opcode.pop = bit::<11>(instruction);
    // return opcode;
    // }
    pub rlist: u8,
    pub pclr: bool,
    pub pop: bool,
}

impl ThumbPushPop {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub struct ThumbLoadStoreSPRelative {
    // static ThumbLoadStoreSPRelative decode(u32 instruction) {
    // ThumbLoadStoreSPRelative opcode;
    // opcode.imm = get_field::<0, 8>(instruction);
    // opcode.rd = static_cast<GPR>(get_field::<8, 3>(instruction));
    // opcode.load = bit::<11>(instruction);
    // return opcode;
    // }
    pub imm: u32,
    pub rd: GPR,
    pub load: bool,
}

impl ThumbLoadStoreSPRelative {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub struct ThumbLoadStoreHalfword {
    // static ThumbLoadStoreHalfword decode(u32 instruction) {
    // ThumbLoadStoreHalfword opcode;
    // opcode.rd = static_cast < GPR> (get_field:: < 0, 3 >(instruction));
    // opcode.rn = static_cast< GPR > (get_field:: < 3, 3 > (instruction));
    // opcode.imm = get_field:: < 6, 5 > (instruction);
    // opcode.load = bit::< 11 > (instruction);
    // return opcode;
    // }
    pub rd: GPR,
    pub rn: GPR,
    pub imm: u32,
    pub load: bool,
}

impl ThumbLoadStoreHalfword {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}

pub struct ThumbLoadStoreMultiple {
    // static ThumbLoadStoreMultiple decode(u32 instruction) {
    // ThumbLoadStoreMultiple opcode;
    // opcode.rlist = get_field::<0, 8>(instruction);
    // opcode.rn = static_cast<GPR>(get_field::<8, 3>(instruction));
    // opcode.load = bit::<11>(instruction);
    // return opcode;
    // }
    pub rlist: u8,
    pub rn: GPR,
    pub load: bool,
}

impl ThumbLoadStoreMultiple {
    pub fn decode(instruction: u32) -> Self {
        todo!()
    }
}
