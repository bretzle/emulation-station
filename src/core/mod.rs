use crate::arm::cpu::Arch;
use log::debug;
use crate::core::arm7::Arm7;

use crate::core::arm9::Arm9;
use crate::core::config::{BootMode, Config};
use crate::core::hardware::cartridge::Cartridge;
use crate::core::hardware::dma::Dma;
use crate::core::hardware::input::Input;
use crate::core::hardware::ipc::Ipc;
use crate::core::hardware::math_unit::MathUnit;
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
    // spu: (),
    dma7: Dma,
    dma9: Dma,
    ipc: Ipc,
    math_unit: MathUnit,
    // rtc: (),
    // spi: (),
    // timer7: (),
    // timer9: (),
    // wifi: (),
    scheduler: Scheduler,

    main_memory: Box<[u8]>,
    // shared_wram: (),
    //
    wramcnt: u8,
    // haltcnt: (),
    // exmemcnt: (),
    // exmemstat: (),
    // rcnt: (),
    config: Config,
}

impl System {
    pub fn new() -> Shared<Self> {
        Shared::new_cyclic(|system| Self {
            arm7: Arm7::new(system),
            arm9: Arm9::new(system),
            cartridge: Cartridge::new(system),
            video_unit: VideoUnit::new(system),
            input: Input::new(),
            dma7: Dma::new(Arch::ARMv4, system),
            dma9: Dma::new(Arch::ARMv5, system),
            ipc: Ipc::new(system),
            math_unit: MathUnit::default(),
            scheduler: Scheduler::new(system),
            main_memory: vec![0; 0x400000].into_boxed_slice(),
            wramcnt: 0,
            config: Config::default(),
        })
    }

    pub fn reset(&mut self) {
        self.arm7.reset();
        self.arm9.reset();
        self.cartridge.load(&self.config.game_path);
        self.video_unit.reset();
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
                cycles = cycles.min(32);
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
        // self.spi.direct_boot();

        debug!("System: direct booted successfully")
    }

    fn write_wramcnt(&mut self, val: u8) {
        self.wramcnt = val & 0x3;
        self.arm9.get_memory().update_wram_mapping();
    }
}
