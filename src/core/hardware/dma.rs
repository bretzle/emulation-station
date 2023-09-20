use crate::arm::cpu::Arch;
use crate::bitfield;
use crate::core::scheduler::EventInfo;
use crate::core::System;
use crate::util::Shared;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq)]
pub enum DmaTiming {
    Immediate = 0,
    VBlank = 1,
    HBlank = 2,
    StartOfDisplay = 3,
    MainMemoryDisplay = 4,
    Slot1 = 5,
    Slot2 = 6,
    GXFIFO = 7,
}

enum AddressMode {
    Increment = 0,
    Decrement = 1,
    Fixed = 2,
    Reload = 3,
}

bitfield! {
    #[derive(Default, Clone)]
    struct Control(u16) {
        // 0 | 4
        destination_control: u8 [AddressMode] => 5 | 6,
        source_control: u8 [AddressMode] => 7 | 8,
        repeat: bool => 9,
        transfer_words: bool => 10,
        timing: u8 [DmaTiming] => 11 | 13,
        irq: bool => 14,
        enable: bool => 15
    }
}

#[derive(Default, Clone)]
struct Channel {
    length: u32,
    source: u32,
    internal_source: u32,
    destination: u32,
    internal_destination: u32,
    internal_length: u32,
    control: Control,
}

pub struct Dma {
    channels: [Channel; 4],
    dmafill: [u32; 4],
    transfer_events: [Rc<EventInfo>; 4],
    system: Shared<System>,
    arch: Arch,
}

impl Dma {
    pub fn new(arch: Arch, system: &Shared<System>) -> Self {
        Self {
            channels: Default::default(),
            dmafill: [0; 4],
            transfer_events: Default::default(),
            system: system.clone(),
            arch,
        }
    }

    pub fn reset(&mut self) {
        self.channels.fill(Channel::default());
        self.dmafill.fill(0);

        for te in &mut self.transfer_events {
            *te = self
                .system
                .scheduler
                .register_event("DMA Transfer", |_| todo!())
        }
    }

    pub fn trigger(&mut self, timing: DmaTiming) {
        for (i, channel) in self.channels.iter_mut().enumerate() {
            let channel_timing = match self.arch {
                Arch::ARMv4 => todo!(),
                Arch::ARMv5 => channel.control.timing(),
            };

            if channel.control.enable() && channel_timing == timing {
                self.system.scheduler.add_event(1, &self.transfer_events[i]);
            }
        }
    }
}
