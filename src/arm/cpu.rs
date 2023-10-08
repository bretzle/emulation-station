use std::fs::File;
use std::io::{BufWriter, Write};
use std::mem::swap;
use std::ops::Not;

use log::{trace, warn};

use crate::arm::coprocessor::Coprocessor;
use crate::arm::decoder::Decoder;
use crate::arm::memory::Memory;
use crate::arm::state::{Bank, Condition, Mode, State, StatusReg, GPR};

#[derive(PartialEq, Copy, Clone, Debug)]
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

pub struct Cpu {
    // common stuff
    pub state: State,
    pub arch: Arch,
    pub memory: Box<dyn Memory>,
    pub coprocessor: Box<dyn Coprocessor>,
    irq: bool,
    halted: bool,

    // interpreter stuff
    decoder: Decoder,
    pipeline: [u32; 2],
    pub instruction: u32,
    condition_table: [[bool; 16]; 16],

    #[cfg(feature = "log_state")]
    debug: BufWriter<File>,
    // jit stuff
    // todo
}

impl Cpu {
    pub fn new(arch: Arch, memory: Box<dyn Memory>, coprocessor: Box<dyn Coprocessor>) -> Self {
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
            #[cfg(feature = "log_state")]
            debug: BufWriter::new(File::create(format!("{arch:?}.log")).unwrap())
        }
    }

    pub fn reset(&mut self) {
        self.state = State::default();
        self.state.cpsr.0 = 0xd3;
        self.switch_mode(Mode::Supervisor);
        self.pipeline.fill(0);
        self.irq = false;
        self.halted = false;
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

    pub fn update_halted(&mut self, val: bool) {
        self.halted = val;
    }

    pub fn run(&mut self, cycles: u64) {
        for _ in 0..cycles {
            if self.halted {
                return;
            }

            if self.irq && !self.state.cpsr.i() {
                self.handle_interrupt();
            }

            self.instruction = self.pipeline[0];
            self.pipeline[0] = self.pipeline[1];

            static mut COUNT: [u32; 2] = [0; 2];
            if self.arch == Arch::ARMv5 && unsafe { COUNT[1] == 254392 } {
                println!("breakpoint")
            }

            if self.state.cpsr.thumb() {
                self.state.gpr[15] &= !0x1;
                self.pipeline[1] = self.code_read_half(self.state.gpr[15]) as u32;
                let handler = self.decoder.decode_thumb(self.instruction);

                (handler)(self, self.instruction);
                self.log_state();
                unsafe { COUNT[self.arch as usize] += 1 }
            } else {
                self.state.gpr[15] &= !0x3;
                self.pipeline[1] = self.code_read_word(self.state.gpr[15]);
                if self.evaluate_cond((self.instruction >> 28).into()) {
                    let handler = self.decoder.decode_arm(self.instruction);
                    (handler)(self, self.instruction);
                    self.log_state();
                    unsafe { COUNT[self.arch as usize] += 1 }
                } else {
                    self.state.gpr[15] += 4;
                }
            }
        }
    }

    #[cfg(feature = "log_state")]
    fn log_state(&mut self) {
        use std::io::Write;

        let thumb = self.state.cpsr.thumb();
        let pc = self.state.gpr[15] - if thumb { 4 } else { 8 };
        let inst = self.instruction;

        writeln!(self.debug, "{pc:08x}: {inst:08x} | {:x?} cpsr: {:08x}", self.state.gpr, self.state.cpsr.0);
    }

    #[cfg(not(feature = "log_state"))]
    fn log_state(&mut self) {}

    fn handle_interrupt(&mut self) {
        self.halted = false;
        self.state.spsr_at(Bank::IRQ).0 = self.state.cpsr.0;
        self.switch_mode(Mode::Irq);
        self.state.cpsr.set_i(true);

        if self.state.cpsr.thumb() {
            self.state.cpsr.set_thumb(false);
            self.state.gpr[14] = self.state.gpr[15];
        } else {
            self.state.gpr[14] = self.state.gpr[15] - 4;
        }

        self.state.gpr[15] = self.coprocessor.get_exception_base() + 0x18;
        self.arm_flush_pipeline();
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
        assert!(self.state.cpsr.thumb());
        self.state.gpr[15] &= !1;
        self.pipeline[0] = self.code_read_half(self.state.gpr[15]) as u32;
        self.pipeline[1] = self.code_read_half(self.state.gpr[15] + 2) as u32;
        self.state.gpr[15] += 4;
    }

    pub fn arm_flush_pipeline(&mut self) {
        assert!(!self.state.cpsr.thumb());
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
