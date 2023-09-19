use log::debug;

use crate::core::arm9::Arm9;
use crate::core::config::{BootMode, Config};
use crate::core::hardware::cartridge::Cartridge;
use crate::core::scheduler::Scheduler;
use crate::util::Shared;

pub mod arm7;
pub mod arm9;
pub mod config;
pub mod hardware;
pub mod scheduler;

pub struct System {
    // arm7: (),
    arm9: Arm9,
    cartridge: Cartridge,
    // video_unit: (),
    // input: (),
    // spu: (),
    // dma7: (),
    // dma9: (),
    // ipc: (),
    // math_unit: (),
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
            arm9: Arm9::new(system),
            cartridge: Cartridge::new(system),
            scheduler: Scheduler::default(),
            main_memory: vec![0; 0x400000].into_boxed_slice(),
            wramcnt: 0,
            config: Config::default(),
        })
    }

    pub fn reset(&mut self) {
        self.cartridge.load(&self.config.game_path);
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
            self.scheduler.tick(cycles);
            self.scheduler.run();
        }
    }

    fn direct_boot(&mut self) {
        self.write_wramcnt(0x03);

        self.cartridge.direct_boot();
        // self.arm7.direct_boot();
        self.arm9.direct_boot();
        // self.spi.direct_boot();

        debug!("System: direct booted successfully")
    }

    fn write_wramcnt(&mut self, val: u8) {
        self.wramcnt = val & 0x3;
        self.arm9.get_memory().update_wram_mapping();
    }
}
