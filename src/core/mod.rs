use log::debug;

use crate::arm::cpu::Arch;
use crate::arm::memory::Memory;
use crate::core::arm7::Arm7;
use crate::core::arm9::Arm9;
use crate::core::config::{BootMode, Config};
use crate::core::hardware::cartridge::Cartridge;
use crate::core::hardware::dma::Dma;
use crate::core::hardware::input::Input;
use crate::core::hardware::ipc::Ipc;
use crate::core::hardware::math_unit::MathUnit;
use crate::core::hardware::rtc::Rtc;
use crate::core::hardware::spi::Spi;
use crate::core::hardware::spu::Spu;
use crate::core::hardware::timer::Timers;
use crate::core::scheduler::Scheduler;
use crate::core::video::VideoUnit;
use crate::util::Shared;

pub mod arm7;
pub mod arm9;
pub mod config;
pub mod hardware;
pub mod scheduler;
pub mod video;

pub struct System {
    arm7: Arm7,
    arm9: Arm9,
    cartridge: Cartridge,
    pub video_unit: VideoUnit,
    pub input: Input,
    spu: Spu,
    dma7: Dma,
    dma9: Dma,
    ipc: Ipc,
    math_unit: MathUnit,
    rtc: Rtc,
    spi: Spi,
    timer7: Timers,
    timer9: Timers,
    // wifi: (),
    scheduler: Scheduler,

    main_memory: Box<[u8]>,
    shared_wram: Box<[u8]>,

    wramcnt: u8,
    haltcnt: u8,
    exmemcnt: u16,
    // exmemstat: (),
    // rcnt: (),
    config: Config,
}

impl System {
    pub fn new() -> Shared<Self> {
        Shared::new_cyclic(|system| {
            let arm7 = Arm7::new(system);
            let arm9 = Arm9::new(system);
            Self {
                cartridge: Cartridge::new(system),
                video_unit: VideoUnit::new(system),
                input: Input::new(),
                spu: Spu::new(),
                dma7: Dma::new(Arch::ARMv4, system),
                dma9: Dma::new(Arch::ARMv5, system),
                ipc: Ipc::new(&arm7.irq, &arm9.irq),
                math_unit: MathUnit::default(),
                rtc: Rtc::new(),
                spi: Spi::new(system),
                timer7: Timers::new(system, &arm7.irq),
                timer9: Timers::new(system, &arm9.irq),
                scheduler: Scheduler::new(system),
                main_memory: vec![0; 0x400000].into_boxed_slice(),
                shared_wram: vec![0; 0x8000].into_boxed_slice(),
                wramcnt: 0,
                haltcnt: 0,
                exmemcnt: 0,
                config: Config::default(),
                arm7,
                arm9,
            }
        })
    }

    pub fn reset(&mut self) {
        self.arm7.reset();
        self.arm9.reset();
        self.cartridge.load(&self.config.game_path);
        self.video_unit.reset();
        self.dma7.reset();
        self.dma9.reset();
        self.spi.reset();
        self.timer7.reset(Arch::ARMv4);
        self.timer9.reset(Arch::ARMv5);
        self.spu.reset();
        self.rtc.reset();
        match self.config.boot_mode {
            BootMode::Firmware => todo!(),
            BootMode::Direct => self.direct_boot(),
        }
    }

    pub fn set_game_path(&mut self, path: &str) {
        self.config.game_path = path.to_string();
    }

    pub fn set_boot_mode(&mut self, boot_mode: BootMode) {
        self.config.boot_mode = boot_mode;
    }

    pub fn run_frame(&mut self) {
        let frame_end = self.scheduler.get_current_time() + 560190;
        while self.scheduler.get_current_time() < frame_end {
            let mut cycles = self.scheduler.get_event_time() - self.scheduler.get_current_time();

            if !self.arm9.is_halted() {
                cycles = cycles.min(16);
            }

            self.arm9.run(2 * cycles);
            self.arm7.run(cycles);
            self.scheduler.tick(cycles);
            self.scheduler.run();
        }

        self.video_unit.ppu_a.on_finish_frame();
        self.video_unit.ppu_b.on_finish_frame();
    }

    // pub fn step(&mut self) {
    //     self.arm9.run(1);
    //     self.scheduler.tick(1);
    //     self.scheduler.run();
    //
    //     self.video_unit.ppu_a.on_finish_frame();
    //     self.video_unit.ppu_b.on_finish_frame();
    // }

    fn direct_boot(&mut self) {
        self.write_wramcnt(0x03);

        self.cartridge.direct_boot();
        self.arm7.direct_boot();
        self.arm9.direct_boot();
        self.spi.direct_boot();

        debug!("System: direct booted successfully")
    }

    fn write_wramcnt(&mut self, val: u8) {
        self.wramcnt = val & 0x3;
        self.arm7.update_wram_mapping();
        self.arm9.update_wram_mapping();
    }

    pub const fn read_wramcnt(&self) -> u8 {
        self.wramcnt
    }

    pub fn get_memory(&mut self, arch: Arch) -> &mut dyn Memory {
        match arch {
            Arch::ARMv4 => self.arm7.get_memory(),
            Arch::ARMv5 => self.arm9.get_memory(),
        }
    }

    pub fn write_haltcnt(&mut self, val: u8) {
        self.haltcnt = val & 0xc0;
        match (self.haltcnt >> 6) & 0x3 {
            0x2 => self.arm7.cpu.update_halted(true),
            0x3 => todo!(),
            _ => todo!(),
        }
    }

    pub const fn read_exmemcnt(&self) -> u16 {
        self.exmemcnt
    }

    pub fn write_exmemcnt(&mut self, val: u16, mask: u16) {
        self.exmemcnt = (self.exmemcnt & !mask) | (val | mask)
    }
}
