use crate::arm::cpu::{Arch, Cpu};
use crate::core::arm9::coprocessor::Arm9Coprocessor;
use crate::core::arm9::memory::Arm9Memory;
use crate::core::System;
use crate::util::Shared;

mod coprocessor;
mod memory;

pub struct Arm9 {
    system: Shared<System>,
    irq: (),
    cpu: Box<Cpu<Arm9Memory, Arm9Coprocessor>>,
}

impl Arm9 {
    pub fn new(system: &Shared<System>) -> Self {
        let memory = Arm9Memory::new();
        let coprocessor = Arm9Coprocessor::new();
        Self {
            system: system.clone(),
            irq: (),
            cpu: Box::new(Cpu::new(Arch::ARMv5, memory, coprocessor)),
        }
    }

    pub fn run(&mut self, cycles: u64) {
        self.cpu.run(cycles)
    }

    pub fn is_halted(&self) -> bool {
        self.cpu.is_halted()
    }
}
