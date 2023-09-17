use crate::arm::decoder::Decoder;
use crate::arm::memory::Memory;
use crate::arm::state::{Condition, State};

#[derive(PartialEq)]
pub enum Arch {
    ARMv4,
    ARMv5,
}

pub struct Cpu<M, C> {
    // common stuff
    pub state: State,
    pub arch: Arch,
    pub memory: M,
    pub coprocessor: C,
    irq: bool,
    halted: bool,

    // interpreter stuff
    decoder: Decoder<M, C>,
    pipeline: [u32; 2],
    instruction: u32,
    condition_table: [[bool; 16]; 16],
    // jit stuff
    // todo
}

impl<M: Memory, C> Cpu<M, C> {
    pub fn new(arch: Arch, memory: M, coprocessor: C) -> Self {
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

    pub(super) fn illegal_instruction(&mut self, _: u32) {
        todo!()
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
                todo!("cpu thumb")
            } else {
                self.state.gpr[15] &= !0x3;
                self.pipeline[1] = self.code_read_word(self.state.gpr[15]);

                if self.evaluate_cond(self.instruction >> 28) {
                    let handler = self.decoder.decode_arm(self.instruction);
                    (handler)(self, self.instruction);
                } else {
                    self.state.gpr[15] += 4;
                }
            }
        }
    }

    fn evaluate_cond(&self, bits: u32) -> bool {
        let cond = Condition::from(bits);
        if cond == Condition::NV {
            return (self.arch == Arch::ARMv5) && (self.instruction & 0x0e000000) == 0xa000000;
        }

        self.condition_table[cond as usize][(self.state.cpsr.bits() >> 28) as usize]
    }

    fn code_read_word(&mut self, addr: u32) -> u32 {
        // todo: self.memory.read::<u32, { Bus::Code }>(addr)
        self.memory.read_word(addr)
    }
}
