use crate::core::arm9::Arm9;
use crate::core::config::{BootMode, Config};
use crate::core::scheduler::Scheduler;
use crate::util::Shared;

pub mod arm7;
pub mod arm9;
pub mod config;
pub mod scheduler;

pub struct System {
    // arm7: (),
    arm9: Arm9,
    // cartridge: (),
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

    // main_memory: (),
    // shared_wram: (),
    //
    // wramcnt: (),
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
            scheduler: Scheduler::default(),
            config: Config::default(),
        })
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
}
