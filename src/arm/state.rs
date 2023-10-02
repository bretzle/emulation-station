use std::mem::transmute;

use crate::bitfield;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Default)]
pub enum GPR {
    #[default]
    R0 = 0,
    R1 = 1,
    R2 = 2,
    R3 = 3,
    R4 = 4,
    R5 = 5,
    R6 = 6,
    R7 = 7,
    R8 = 8,
    R9 = 9,
    R10 = 10,
    R11 = 11,
    R12 = 12,
    SP = 13,
    LR = 14,
    PC = 15,
}

impl From<u32> for GPR {
    fn from(value: u32) -> Self {
        unsafe { transmute(value as u8) }
    }
}

#[repr(u8)]
#[derive(PartialEq, Copy, Clone)]
pub enum Bank {
    USR = 0,
    FIQ = 1,
    IRQ = 2,
    SVC = 3,
    ABT = 4,
    UND = 5,
    CPSR = u8::MAX,
}

#[repr(u8)]
#[derive(PartialEq, Default)]
pub enum Condition {
    #[default]
    EQ = 0,
    NE = 1,
    CS = 2,
    CC = 3,
    MI = 4,
    PL = 5,
    VS = 6,
    VC = 7,
    HI = 8,
    LS = 9,
    GE = 10,
    LT = 11,
    GT = 12,
    LE = 13,
    AL = 14,
    NV = 15,
}

impl From<u32> for Condition {
    fn from(value: u32) -> Self {
        unsafe { transmute(value as u8) }
    }
}

impl Condition {
    pub fn table() -> [[bool; 16]; 16] {
        use Condition::*;
        let mut table = [[false; 16]; 16];

        for i in 0..16 {
            let n = i & 8 != 0;
            let z = i & 4 != 0;
            let c = i & 2 != 0;
            let v = i & 1 != 0;

            table[EQ as usize][i] = z;
            table[NE as usize][i] = !z;
            table[CS as usize][i] = c;
            table[CC as usize][i] = !c;
            table[MI as usize][i] = n;
            table[PL as usize][i] = !n;
            table[VS as usize][i] = v;
            table[VC as usize][i] = !v;
            table[HI as usize][i] = c && !z;
            table[LS as usize][i] = !c || z;
            table[GE as usize][i] = n == v;
            table[LT as usize][i] = n != v;
            table[GT as usize][i] = !z && (n == v);
            table[LE as usize][i] = z || (n != v);
            table[AL as usize][i] = true;

            // this one is architecture and instruction dependent
            table[NV as usize][i] = true;
        }

        table
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Mode {
    User = 0x10,
    Fiq = 0x11,
    Irq = 0x12,
    Supervisor = 0x13,
    Abort = 0x17,
    Undefined = 0x1b,
    System = 0x1f,
}

impl From<u32> for Mode {
    fn from(value: u32) -> Self {
        unsafe { transmute(value as u8) }
    }
}

impl Mode {
    pub fn bank(self) -> Bank {
        match self {
            Mode::User | Mode::System => Bank::USR,
            Mode::Fiq => Bank::FIQ,
            Mode::Irq => Bank::IRQ,
            Mode::Supervisor => Bank::SVC,
            Mode::Abort => Bank::ABT,
            Mode::Undefined => Bank::UND,
        }
    }
}

bitfield! {
    #[derive(Default, Copy, Clone)]
    pub struct StatusReg(pub u32) {
        pub mode: u8 [Mode] => 0 | 4,
        pub thumb: bool => 5,
        pub f: bool => 6,
        pub i: bool => 7,
        pub q: bool => 27,
        pub v: bool => 28,
        pub c: bool => 29,
        pub z: bool => 30,
        pub n: bool => 31
    }
}

#[derive(Default)]
pub struct State {
    pub gpr: [u32; 16],
    pub gpr_banked: [[u32; 7]; 6],
    pub cpsr: StatusReg,
    spsr: usize,
    spsr_banked: [StatusReg; 6],
}

impl State {
    #[inline]
    pub fn spsr(&self) -> &StatusReg {
        self.spsr_banked.get(self.spsr).unwrap_or(&self.cpsr)
    }

    #[inline]
    pub fn spsr_mut(&mut self) -> &mut StatusReg {
        self.spsr_banked.get_mut(self.spsr).unwrap_or(&mut self.cpsr)
    }

    pub fn spsr_at(&mut self, bank: Bank) -> &mut StatusReg {
        &mut self.spsr_banked[bank as usize]
    }

    pub fn set_spsr(&mut self, bank: Bank) {
        self.spsr = bank as usize;
    }
}
