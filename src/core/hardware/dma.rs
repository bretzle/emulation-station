use std::mem::transmute;
use std::ops::Shr;
use std::rc::Rc;

use crate::arm::cpu::Arch;
use crate::bitfield;
use crate::core::scheduler::EventInfo;
use crate::core::System;
use crate::util::{set, Shared};

const ADJUST_LUT: [[i32; 4]; 2] = [[2, -2, 0, 2], [4, -4, 0, 4]];

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

impl Shr<usize> for DmaTiming {
    type Output = DmaTiming;

    fn shr(self, rhs: usize) -> Self::Output {
        unsafe {
            transmute(self as u8 >> rhs)
        }
    }
}

enum AddressMode {
    Increment = 0,
    Decrement = 1,
    Fixed = 2,
    Reload = 3,
}

bitfield! {
    #[derive(Default, Copy, Clone)]
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

    #[rustfmt::skip]
    pub fn reset(&mut self) {
        self.channels.fill(Channel::default());
        self.dmafill.fill(0);

        match self.arch {
            Arch::ARMv4 => {
                self.transfer_events[0] = self.system.scheduler.register_event("DMA Transfer 7.0", |system| system.dma7.transfer(0));
                self.transfer_events[1] = self.system.scheduler.register_event("DMA Transfer 7.1", |system| system.dma7.transfer(1));
                self.transfer_events[2] = self.system.scheduler.register_event("DMA Transfer 7.2", |system| system.dma7.transfer(2));
                self.transfer_events[3] = self.system.scheduler.register_event("DMA Transfer 7.3", |system| system.dma7.transfer(3));
            }
            Arch::ARMv5 => {
                self.transfer_events[0] = self.system.scheduler.register_event("DMA Transfer 9.0", |system| system.dma9.transfer(0));
                self.transfer_events[1] = self.system.scheduler.register_event("DMA Transfer 9.1", |system| system.dma9.transfer(1));
                self.transfer_events[2] = self.system.scheduler.register_event("DMA Transfer 9.2", |system| system.dma9.transfer(2));
                self.transfer_events[3] = self.system.scheduler.register_event("DMA Transfer 9.3", |system| system.dma9.transfer(3));
            }
        }
    }

    pub fn trigger(&mut self, timing: DmaTiming) {
        for (i, channel) in self.channels.iter_mut().enumerate() {
            let channel_timing = match self.arch {
                Arch::ARMv4 => channel.control.timing() >> 1,
                Arch::ARMv5 => channel.control.timing(),
            };

            if channel.control.enable() && channel_timing == timing {
                self.system.scheduler.add_event(1, &self.transfer_events[i]);
            }
        }
    }

    pub fn transfer(&mut self, id: usize) {
        let channel = &mut self.channels[id];
        let source_adjust = ADJUST_LUT[channel.control.transfer_words() as usize][channel.control.source_control() as usize];
        let dest_adjust = ADJUST_LUT[channel.control.transfer_words() as usize][channel.control.destination_control() as usize];

        if channel.control.transfer_words() {
            for _ in 0..channel.internal_length {
                let mem = self.system.get_memory(self.arch);
                let val = mem.read_word(channel.internal_source);
                mem.write_word(channel.internal_destination, val);

                channel.internal_source += source_adjust as u32;
                channel.internal_destination += dest_adjust as u32;
            }
        } else {
            for _ in 0..channel.internal_length {
                let mem = self.system.get_memory(self.arch);
                let val = mem.read_half(channel.internal_source);
                mem.write_half(channel.internal_destination, val);

                channel.internal_source += source_adjust as u32;
                channel.internal_destination += dest_adjust as u32;
            }
        }

        if channel.control.irq() {
            todo!()
        }

        if channel.control.repeat() && channel.control.timing() != DmaTiming::Immediate {
            todo!()
        } else {
            channel.control.set_enable(false);
        }
    }

    pub fn write_source(&mut self, id: usize, val: u32, mask: u32) {
        self.channels[id].source = (self.channels[id].source & !mask) | (val & mask);
    }

    pub fn write_destination(&mut self, id: usize, val: u32, mask: u32) {
        self.channels[id].destination = (self.channels[id].destination & !mask) | (val & mask);
    }

    pub fn write_length(&mut self, id: usize, val: u32, mask: u32) {
        self.channels[id].length = (self.channels[id].length & !mask) | (val & mask);
    }

    pub fn write_control(&mut self, id: usize, val: u32, mask: u32) {
        let channel = &mut self.channels[id];
        let old = channel.control;

        channel.length &= 0xffff;
        channel.length |= (val & 0x1f & mask) << 16;
        set(&mut channel.control.0, val as u16, mask as u16);

        if channel.control.enable() && channel.control.timing() == DmaTiming::GXFIFO {
            todo!()
        }

        if old.enable() || !channel.control.enable() {
            return;
        }

        channel.internal_source = channel.source;
        channel.internal_destination = channel.destination;

        if channel.length == 0 {
            if self.arch == Arch::ARMv5 {
                channel.internal_length = 0x200000
            } else {
                channel.internal_length = 0x10000
            }
        } else {
            channel.internal_length = channel.length
        }

        if channel.control.timing() == DmaTiming::Immediate {
            self.system.scheduler.add_event(1, &self.transfer_events[id])
        }
    }

    pub fn write_dmafill(&mut self, addr: u32, val: u32) {
        self.dmafill[((addr - 0x040000e0) / 4) as usize] = val
    }

    pub const fn read_source(&self, id: usize) -> u32 {
        self.channels[id].source
    }

    pub const fn read_length(&self, id: usize) -> u32 {
        self.channels[id].length
    }

    pub const fn read_control(&self, id: usize) -> u16 {
        let left = ((self.channels[id].length >> 16) & 0x1f) as u16;
        let right = self.channels[id].control.0;
        left | right
    }

    pub const fn read_dmafill(&self, addr: u32) -> u32 {
        self.dmafill[((addr - 0x040000e0) / 4) as usize]
    }
}
