use std::any::Any;
use crate::arm::coprocessor::Coprocessor;
use crate::arm::cpu::{Arch, Cpu};
use crate::arm::memory::Memory;
use crate::arm::state::{Mode, StatusReg, GPR};
use crate::core::arm9::coprocessor::Arm9Coprocessor;
use crate::core::arm9::memory::Arm9Memory;
use crate::core::hardware::irq::Irq;
use crate::core::System;
use crate::util::Shared;

mod coprocessor;
mod memory;

pub struct Arm9 {
    system: Shared<System>,
    irq: Irq,
    pub cpu: Shared<Cpu>,
}

impl Arm9 {
    pub fn new(system: &Shared<System>) -> Self {
        let memory = Box::new(Arm9Memory::new(system));
        let coprocessor = Box::new(Arm9Coprocessor::new(&memory.itcm, &memory.dtcm));
        let cpu = Shared::new(Cpu::new(Arch::ARMv5, memory, coprocessor));
        Self {
            system: system.clone(),
            irq: Irq::new(&cpu),
            cpu,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.memory.reset();
    }

    pub fn run(&mut self, cycles: u64) {
        self.cpu.run(cycles)
    }

    pub fn is_halted(&self) -> bool {
        self.cpu.is_halted()
    }

    pub fn direct_boot(&mut self) {
        self.get_memory().write_byte(0x04000300, 0x01); // postflg (arm9)
        self.get_memory().write_half(0x04000304, 0x0001); // powcnt1
        self.get_memory().write_word(0x027ff800, 0x00001fc2); // chip id 1
        self.get_memory().write_word(0x027ff804, 0x00001fc2); // chip id 2
        self.get_memory().write_half(0x027ff850, 0x5835); // arm7 bios crc
        self.get_memory().write_half(0x027ff880, 0x0007); // message from arm9 to arm7
        self.get_memory().write_half(0x027ff884, 0x0006); // arm7 boot task
        self.get_memory().write_word(0x027ffc00, 0x00001fc2); // copy of chip id 1
        self.get_memory().write_word(0x027ffc04, 0x00001fc2); // copy of chip id 2
        self.get_memory().write_half(0x027ffc10, 0x5835); // copy of arm7 bios crc
        self.get_memory().write_half(0x027ffc40, 0x0001); // boot indicator

        self.get_coprocessor().write(1, 0, 0, 0x0005707d);
        self.get_coprocessor().write(9, 1, 0, 0x0300000a);
        self.get_coprocessor().write(9, 1, 1, 0x00000020);

        // enter system mode
        self.cpu.set_cpsr(StatusReg(0xdf));

        use GPR::*;
        let entrypoint = self.system.cartridge.get_arm9_entrypoint();
        self.cpu.set_gpr(R12, entrypoint);
        self.cpu.set_gpr(SP, 0x03002f7c);
        self.cpu.set_gpr_banked(SP, Mode::Irq, 0x03003f80);
        self.cpu.set_gpr_banked(SP, Mode::Supervisor, 0x03003fc0);
        self.cpu.set_gpr(LR, entrypoint);
        self.cpu.set_gpr(PC, entrypoint);
    }

    pub fn get_memory(&mut self) -> &mut dyn Memory {
        &mut *self.cpu.memory
    }

    pub fn get_coprocessor(&mut self) -> &mut dyn Coprocessor {
        &mut *self.cpu.coprocessor
    }

    pub fn get_irq(&mut self) -> &mut Irq {
        &mut self.irq
    }

    pub fn update_wram_mapping(&mut self) {
        self.cpu.memory.as_any().downcast_mut::<Arm9Memory>().unwrap().update_wram_mapping()
    }
}
