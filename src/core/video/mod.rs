use crate::core::video::vram::Vram;
use crate::core::System;
use crate::util::Shared;

pub mod vram;

pub struct VideoUnit {
    system: Shared<System>,
    pub vram: Vram,
    pub ppu_a: (),
    pub ppu_b: (),
    pub gpu: (),

    palette_ram: [u8; 0x800],
    oam: [u8; 0x800],

    powcnt1: (),
    vcount: (),
    dispstat7: (),
    dispstat9: (),
    dispcapcnt: (),
    irq7: (),
    irq9: (),
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
            powcnt1: (),
            vcount: (),
            dispstat7: (),
            dispstat9: (),
            dispcapcnt: (),
            irq7: (),
            irq9: (),
        }
    }
}
