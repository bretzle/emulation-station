use std::rc::Rc;

use crate::bitfield;
use crate::core::scheduler::EventInfo;
use crate::core::video::vram::Vram;
use crate::core::System;
use crate::util::Shared;

pub mod vram;

bitfield! {
    struct PowCnt1(u32) {
        enable_both_lcds: bool => 0,
        enable_engine_a: bool => 1,
        enable_rendering_engine: bool => 2,
        enable_geometry_engine: bool => 3,
        // 4 | 8
        enable_engine_b: bool => 9,
        // 10 | 14
        display_swap: bool => 15
    }
}

pub struct VideoUnit {
    system: Shared<System>,
    pub vram: Vram,
    pub ppu_a: (),
    pub ppu_b: (),
    pub gpu: (),

    palette_ram: [u8; 0x800],
    oam: [u8; 0x800],

    powcnt1: PowCnt1,
    vcount: (),
    dispstat7: (),
    dispstat9: (),
    dispcapcnt: (),
    irq7: (),
    irq9: (),

    scanline_start_event: Rc<EventInfo>,
    scanline_end_event: Rc<EventInfo>,
}

impl VideoUnit {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            vram: Vram::new(),
            ppu_a: (),
            ppu_b: (),
            gpu: (),
            palette_ram: [0; 0x800],
            oam: [0; 0x800],
            powcnt1: PowCnt1(0),
            vcount: (),
            dispstat7: (),
            dispstat9: (),
            dispcapcnt: (),
            irq7: (),
            irq9: (),

            scanline_start_event: Rc::default(),
            scanline_end_event: Rc::default(),
        }
    }

    pub fn reset(&mut self) {
        self.palette_ram.fill(0);
        self.oam.fill(0);
        self.powcnt1.0 = 0;

        self.vram.reset();
        // todo: reset other stuff

        let scheduler = &mut self.system.scheduler;
        self.scanline_start_event = scheduler.register_event("Scanline Start", |system| {
            system.video_unit.render_scanline_start();
            system
                .scheduler
                .add_event(524, &system.video_unit.scanline_end_event);
        });
        self.scanline_end_event = scheduler.register_event("Scanline End", |system| {
            system.video_unit.render_scanline_end();
            system
                .scheduler
                .add_event(1606, &system.video_unit.scanline_start_event);
        });

        scheduler.add_event(1606, &self.scanline_start_event);
    }

    pub fn write_powcnt1(&mut self, val: u32, mut mask: u32) {
        mask &= 0x820f;
        self.powcnt1.0 = (self.powcnt1.0 & !mask) | (val & mask);
    }

    fn render_scanline_start(&mut self) {}

    fn render_scanline_end(&mut self) {}
}
