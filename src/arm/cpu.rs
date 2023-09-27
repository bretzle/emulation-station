use std::ops::Not;

use log::warn;

use crate::arm::coprocessor::Coprocessor;
use crate::arm::decoder::Decoder;
use crate::arm::memory::Memory;
use crate::arm::state::{Bank, Condition, Mode, State, StatusReg, GPR};
use crate::util::Shared;

#[derive(PartialEq, Copy, Clone)]
pub enum Arch {
    ARMv4,
    ARMv5,
}

impl Not for Arch {
    type Output = Arch;

    fn not(self) -> Self::Output {
        match self {
            Arch::ARMv4 => Arch::ARMv5,
            Arch::ARMv5 => Arch::ARMv4,
        }
    }
}

pub struct Cpu<M, C> {
    // common stuff
    pub state: State,
    pub arch: Arch,
    pub memory: Shared<M>,
    pub coprocessor: C,
    irq: bool,
    halted: bool,

    // interpreter stuff
    decoder: Decoder<M, C>,
    pipeline: [u32; 2],
    pub instruction: u32,
    condition_table: [[bool; 16]; 16],
    // jit stuff
    // todo
}

impl<M: Memory, C: Coprocessor> Cpu<M, C> {
    pub fn new(arch: Arch, memory: Shared<M>, coprocessor: C) -> Self {
        Self {
            state: State::default(),
            arch,
            memory,
            coprocessor,
            irq: false,
            halted: false,
            decoder: Decoder::new(),
            pipeline: [0; 2],
            instruction: 0,
            condition_table: Condition::table(),
        }
    }

    pub(super) fn illegal_instruction(&mut self, instruction: u32) {
        panic!(
            "Interpreter: illegal instruction {instruction:08x} at pc = {:08x}",
            self.state.gpr[15]
        );
    }

    pub const fn is_halted(&self) -> bool {
        self.halted
    }

    pub fn run(&mut self, cycles: u64) {
        for _ in 0..cycles {
            if self.halted {
                return;
            }

            if self.irq && !self.state.cpsr.i() {
                todo!("handle interrupts")
            }

            self.instruction = self.pipeline[0];
            self.pipeline[0] = self.pipeline[1];

            if self.state.cpsr.thumb() {
                self.state.gpr[15] &= !0x1;
                self.pipeline[1] = self.code_read_half(self.state.gpr[15]) as u32;
                let handler = self.decoder.decode_thumb(self.instruction);
                (handler)(self, self.instruction)
            } else {
                self.state.gpr[15] &= !0x3;
                self.pipeline[1] = self.code_read_word(self.state.gpr[15]);
                if self.evaluate_cond((self.instruction >> 28).into()) {
                    let handler = self.decoder.decode_arm(self.instruction);
                    if self.state.gpr[15] - 8 == 0x2007b60 || self.state.gpr[15] - 4 == 0x2007b5c {
                        println!("")
                    }
                    (handler)(self, self.instruction);
                } else {
                    self.state.gpr[15] += 4;
                }
            }
        }
    }

    pub fn evaluate_cond(&self, cond: Condition) -> bool {
        if cond == Condition::NV {
            return (self.arch == Arch::ARMv5) && (self.instruction & 0x0e000000) == 0xa000000;
        }

        self.condition_table[cond as usize][(self.state.cpsr.bits() >> 28) as usize]
    }

    pub fn get_cpsr(&self) -> StatusReg {
        self.state.cpsr
    }

    pub fn set_cpsr(&mut self, val: StatusReg) {
        self.state.cpsr = val;
    }

    pub fn set_gpr(&mut self, reg: GPR, val: u32) {
        self.state.gpr[reg as usize] = val;
        if reg == GPR::PC {
            if self.state.cpsr.thumb() {
                self.thumb_flush_pipeline();
            } else {
                self.arm_flush_pipeline();
            }
        }
    }

    pub fn set_gpr_banked(&mut self, gpr: GPR, mode: Mode, value: u32) {
        let start = if mode == Mode::Fiq { GPR::R8 } else { GPR::SP };
        if self.state.cpsr.mode() != mode && gpr >= start && gpr <= GPR::LR {
            self.state.gpr_banked[mode.bank() as usize][gpr as usize - 8] = value;
        } else {
            self.set_gpr(gpr, value);
        }
    }

    pub fn thumb_flush_pipeline(&mut self) {
        self.state.gpr[15] &= !1;
        self.pipeline[0] = self.code_read_half(self.state.gpr[15]) as u32;
        self.pipeline[1] = self.code_read_half(self.state.gpr[15] + 2) as u32;
        self.state.gpr[15] += 4;
    }

    pub fn arm_flush_pipeline(&mut self) {
        self.state.gpr[15] &= !3;
        self.pipeline[0] = self.code_read_word(self.state.gpr[15]);
        self.pipeline[1] = self.code_read_word(self.state.gpr[15] + 4);
        self.state.gpr[15] += 8;
    }

    fn code_read_half(&mut self, addr: u32) -> u16 {
        // todo: self.memory.read::<u16, { Bus::Code }>(addr)
        self.memory.read_half(addr)
    }

    fn code_read_word(&mut self, addr: u32) -> u32 {
        // todo: self.memory.read::<u32, { Bus::Code }>(addr)
        self.memory.read_word(addr)
    }

    pub fn read_word_rotate(&mut self, addr: u32) -> u32 {
        let val = self.memory.read_word(addr);
        let amount = (addr & 0x3) * 8;
        val.rotate_right(amount)
    }

    pub fn undefined_exception(&mut self) {
        warn!(
            "Interpreter: undefined exception fired for instruction {:08x} at {:08x}",
            self.instruction, self.state.gpr[15]
        );

        *self.state.spsr_at(Bank::UND) = self.state.cpsr;
        self.switch_mode(Mode::Undefined);

        self.state.cpsr.set_i(true);
        self.state.gpr[14] = self.state.gpr[15] - 4;
        self.state.gpr[15] = self.coprocessor.get_exception_base() + 0x04;
        self.arm_flush_pipeline();
    }

    pub fn switch_mode(&mut self, mode: Mode) {
        let old = self.state.cpsr.mode().bank();
        let new = mode.bank();

        if new != Bank::USR {
            self.state.set_spsr(new);
        } else {
            self.state.set_spsr(Bank::CPSR);
        }

        self.state.cpsr.set_mode(mode);

        if old == Bank::FIQ || new == Bank::FIQ {
            for i in 0..7 {
                self.state.gpr_banked[old as usize][i] = self.state.gpr[i + 8];
            }
            for i in 0..7 {
                self.state.gpr[i + 8] = self.state.gpr_banked[new as usize][i];
            }
        } else {
            self.state.gpr_banked[old as usize][5] = self.state.gpr[13];
            self.state.gpr_banked[old as usize][6] = self.state.gpr[14];

            self.state.gpr[13] = self.state.gpr_banked[new as usize][5];
            self.state.gpr[14] = self.state.gpr_banked[new as usize][6];
        }
    }

    pub fn set_nz(&mut self, res: u32) {
        self.state.cpsr.set_n(res >> 31 != 0);
        self.state.cpsr.set_z(res == 0);
    }

    pub fn update_irq(&mut self, irq: bool) {
        self.irq = irq;
    }
}
