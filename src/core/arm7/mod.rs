use crate::arm::cpu::{Arch, Cpu};
use crate::arm::memory::Memory;
use crate::arm::state::{Mode, StatusReg, GPR, Bank};
use crate::core::arm7::coprocessor::Arm7Coprocessor;
use crate::core::arm7::memory::Arm7Memory;
use crate::core::hardware::irq::Irq;
use crate::core::System;
use crate::util::Shared;

mod coprocessor;
mod memory;

pub struct Arm7 {
    system: Shared<System>,
    pub irq: Shared<Irq>,
    pub cpu: Shared<Cpu>,
}

impl Arm7 {
    pub fn new(system: &Shared<System>) -> Self {
        let memory = Box::new(Arm7Memory::new(system));
        let coprocessor = Box::new(Arm7Coprocessor);
        let cpu = Shared::new(Cpu::new(Arch::ARMv4, memory, coprocessor));
        Self {
            system: system.clone(),
            irq: Shared::new(Irq::new(&cpu)),
            cpu,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.memory.reset();
        self.cpu.reset();
    }

    pub fn run(&mut self, cycles: u64) {
        self.cpu.run(cycles)
    }

    pub fn direct_boot(&mut self) {
        self.get_memory().write_half(0x04000134, 0x8000); // rcnt
        self.get_memory().write_byte(0x04000300, 0x01); // postflg (arm7)
        self.get_memory().write_half(0x04000504, 0x0200); // soundbias

        // enter system mode
        // self.cpu.set_cpsr(StatusReg(0xdf));

        use GPR::*;
        let entrypoint = self.system.cartridge.get_arm7_entrypoint();
        // self.cpu.set_gpr(R12, entrypoint);
        // self.cpu.set_gpr(SP, 0x0380fd80);
        // self.cpu.set_gpr_banked(SP, Mode::Irq, 0x0380ff80);
        // self.cpu.set_gpr_banked(SP, Mode::Supervisor, 0x0380ffc0);
        // self.cpu.set_gpr(LR, entrypoint);
        // self.cpu.set_gpr(PC, entrypoint);

        self.cpu.state.gpr[12] = entrypoint;
        self.cpu.state.gpr[14] = entrypoint;
        self.cpu.state.gpr[15] = entrypoint;

        self.cpu.state.gpr[13] = 0x0380fd80;
        self.cpu.state.gpr_banked[Bank::IRQ as usize][5] = 0x0380ff80;
        self.cpu.state.gpr_banked[Bank::SVC as usize][5] = 0x0380ffc0;

        self.cpu.set_cpsr(StatusReg(0xdf));
        self.cpu.switch_mode(Mode::System);
        self.cpu.arm_flush_pipeline();
    }

    pub fn get_memory(&mut self) -> &mut dyn Memory {
        &mut *self.cpu.memory
    }
    pub fn get_irq(&mut self) -> &mut Irq {
        &mut self.irq
    }

    pub fn update_wram_mapping(&mut self) {
        self.cpu.memory.as_any().downcast_mut::<Arm7Memory>().unwrap().update_wram_mapping()
    }
}
