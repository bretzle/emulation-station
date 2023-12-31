use crate::arm::cpu::Arch;
use std::rc::Rc;
use std::sync::Arc;
use log::error;

use crate::bitfield;
use crate::core::hardware::dma::DmaTiming;
use crate::core::hardware::irq::{Irq, IrqSource};
use crate::core::scheduler::EventInfo;
use crate::core::video::ppu::Ppu;
use crate::core::video::vram::{Vram, VramBank};
use crate::core::System;
use crate::util::{set, Shared};

pub mod ppu;
pub mod vram;

pub enum Screen {
    Top,
    Bottom,
}

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

bitfield! {
    struct DispStat(u32) {
        vblank: bool => 0,
        hblank: bool => 1,
        lyc: bool => 2,
        vblank_irq: bool => 3,
        hblank_irq: bool => 4,
        lyc_irq: bool => 5,
        // 6
        lyc_setting_msb: u16 => 7,
        lyc_setting: u16 => 8 | 15
    }
}

bitfield! {
    struct DispCapCnt(u32) {
         eva: u32 => 0 | 4,
         // 5 | 7
         evb: u32 => 8 | 12,
         // 13 | 15
         vram_write_block: u32 => 16 | 17,
         vram_write_offset: u32 => 18 | 19,
         capture_size: u32 => 20 | 21,
         // 22 | 23
         source_a: bool => 24,
         source_b: bool => 25,
         vram_read_offset: u32 => 26 | 27,
         // 28
         capture_source: u32 => 29 | 30,
         capture_enable: bool => 31
    }
}

pub struct VideoUnit {
    system: Shared<System>,
    pub vram: Vram,
    pub ppu_a: Ppu,
    pub ppu_b: Ppu,
    pub gpu: (),

    palette_ram: Box<[u8; 0x800]>,
    oam: Box<[u8; 0x800]>,

    powcnt1: PowCnt1,
    vcount: u16,
    dispstat7: DispStat,
    dispstat9: DispStat,
    dispcapcnt: DispCapCnt,
    irq7: Shared<Irq>,
    irq9: Shared<Irq>,

    scanline_start_event: Rc<EventInfo>,
    scanline_end_event: Rc<EventInfo>,
}

impl VideoUnit {
    pub fn new(system: &Shared<System>, irq7: &Shared<Irq>, irq9: &Shared<Irq>) -> Self {
        let vram = Vram::new();
        let mut palette_ram = Box::new([0; 0x800]);
        let mut oam = Box::new([0; 0x800]);
        Self {
            system: system.clone(),
            ppu_a: Ppu::new(
                &vram.bga,
                &vram.obja,
                &vram.bga_extended_palette,
                &vram.obja_extended_palette,
                &vram.lcdc,
                palette_ram.as_mut_slice(),
                oam.as_mut_slice()
            ),
            ppu_b: Ppu::new(
                &vram.bgb,
                &vram.objb,
                &vram.bgb_extended_palette,
                &vram.objb_extended_palette,
                &vram.lcdc,
                &mut palette_ram.as_mut_slice()[0x400..],
                &mut oam.as_mut_slice()[0x400..]
            ),
            vram,
            gpu: (),
            palette_ram,
            oam,
            powcnt1: PowCnt1(0),
            vcount: 0,
            dispstat7: DispStat(0),
            dispstat9: DispStat(0),
            dispcapcnt: DispCapCnt(0),
            irq7: irq7.clone(),
            irq9: irq9.clone(),

            scanline_start_event: Rc::default(),
            scanline_end_event: Rc::default(),
        }
    }

    pub fn reset(&mut self) {
        self.palette_ram.fill(0);
        self.oam.fill(0);
        self.powcnt1.0 = 0;
        self.dispstat7.0 = 0;
        self.dispstat9.0 = 0;
        self.vcount = 0;

        self.vram.reset();
        self.ppu_a.reset();
        self.ppu_b.reset();

        let scheduler = &mut self.system.scheduler;
        self.scanline_start_event = scheduler.register_event("Scanline Start", |system| {
            system.video_unit.render_scanline_start();
            system.scheduler.add_event(524, &system.video_unit.scanline_end_event);
        });
        self.scanline_end_event = scheduler.register_event("Scanline End", |system| {
            system.video_unit.render_scanline_end();
            system.scheduler.add_event(1606, &system.video_unit.scanline_start_event);
        });

        scheduler.add_event(1606, &self.scanline_start_event);
    }

    pub fn fetch_framebuffer(&self, screen: Screen) -> &[u8] {
        if self.powcnt1.display_swap() == matches!(screen, Screen::Top) {
            self.ppu_a.fetch_framebuffer()
        } else {
            self.ppu_b.fetch_framebuffer()
        }
    }

    fn render_scanline_start(&mut self) {
        if self.vcount < 192 {
            self.ppu_a.render_scanline(self.vcount);
            self.ppu_b.render_scanline(self.vcount);
            self.system.dma9.trigger(DmaTiming::HBlank);
        }

        self.dispstat7.set_hblank(true);
        self.dispstat9.set_hblank(true);

        if self.dispstat7.hblank_irq() {
            todo!()
        }

        if self.dispstat9.hblank_irq() {
            todo!()
        }

        // todo: 3d rendering

        if self.vcount > 1 && self.vcount < 194 {
            self.system.dma9.trigger(DmaTiming::StartOfDisplay)
        }
    }

    fn render_scanline_end(&mut self) {
        self.vcount += 1;
        if self.vcount == 263 {
            self.vcount = 0;
        }

        self.dispstat7.set_hblank(false);
        self.dispstat9.set_hblank(false);

        if self.vcount == 192 {
            self.dispstat7.set_vblank(true);
            self.dispstat9.set_vblank(true);

            if self.dispstat7.vblank_irq() {
                self.irq7.raise(IrqSource::VBlank)
            }
            if self.dispstat9.vblank_irq() {
                self.irq9.raise(IrqSource::VBlank)
            }

            self.system.dma9.trigger(DmaTiming::VBlank);
        } else if self.vcount == 262 {
            self.dispstat7.set_vblank(false);
            self.dispstat9.set_vblank(false);
        }

        if self.dispstat7.lyc_setting() | self.dispstat7.lyc_setting_msb() << 1 == self.vcount {
            self.dispstat7.set_lyc(true);
            if self.dispstat7.lyc_irq() {
                self.irq7.raise(IrqSource::VCounter)
            }
        } else {
            self.dispstat7.set_lyc(false);
        }

        if self.dispstat9.lyc_setting() | self.dispstat9.lyc_setting_msb() << 1 == self.vcount {
            self.dispstat9.set_lyc(true);
            if self.dispstat9.lyc_irq() {
                self.irq9.raise(IrqSource::VCounter)
            }
        } else {
            self.dispstat9.set_lyc(false);
        }
    }
}

// mmio
impl VideoUnit {
    pub fn read_dispstat(&mut self, arch: Arch) -> u32 {
        match arch {
            Arch::ARMv4 => self.dispstat7.0,
            Arch::ARMv5 => self.dispstat9.0,
        }
    }

    pub fn read_vcount(&mut self) -> u32 {
        self.vcount as u32
    }

    pub const fn read_powcnt1(&self) -> u32 {
        self.powcnt1.0
    }

    pub fn write_powcnt1(&mut self, val: u32, mut mask: u32) {
        mask &= 0x820f;
        self.powcnt1.0 = (self.powcnt1.0 & !mask) | (val & mask);
    }

    pub fn write_oam<T>(&mut self, addr: u32, val: T) {
        unsafe { std::ptr::write(self.oam.as_mut_ptr().add((addr & 0x7ff) as usize).cast(), val) }
    }

    pub fn write_palette_ram<T>(&mut self, addr: u32, val: T) {
        unsafe { std::ptr::write(self.palette_ram.as_mut_ptr().add((addr & 0x7ff) as usize).cast(), val) }
    }

    pub fn write_dispstat(&mut self, arch: Arch, val: u32, mask: u32) {
        let mask = mask & 0xffbf;
        match arch {
            Arch::ARMv4 => self.dispstat7.0 = (self.dispstat7.0 & !mask) | (val & mask),
            Arch::ARMv5 => self.dispstat9.0 = (self.dispstat9.0 & !mask) | (val & mask),
        }
    }

    pub fn write_vcount(&mut self, val: u16, mask: u16) {
        self.vcount = (self.vcount & !mask) | (val & mask)
    }

    pub fn write_dispcapcnt(&mut self, val: u32, mask: u32) {
        set(&mut self.dispcapcnt.0, val, mask);
        if self.dispcapcnt.capture_enable() {
            error!("VideoUnit: handle display capture")
        }
    }
}
