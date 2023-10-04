use std::rc::Rc;
use log::error;

use crate::arm::cpu::Arch;
use crate::bitfield;
use crate::core::hardware::irq::{Irq, IrqSource};
use crate::core::scheduler::EventInfo;
use crate::core::System;
use crate::util::Shared;

const SHIFTS: [u32; 4] = [0, 6, 8, 10];

bitfield! {
    #[derive(Default, Clone, Copy)]
    struct Control(u16) {
        prescaler: usize => 0 | 1,
        count_up: bool => 2,
        // 3 | 5
        irq: bool => 6,
        start: bool => 7
        // 8 | 15
    }
}

#[derive(Default)]
struct Channel {
    control: Control,
    counter: u32,
    reload_value: u32,
    activation_timestamp: u64,
    active: bool,
    shift: u32,
}

pub struct Timers {
    system: Shared<System>,
    irq: Shared<Irq>,
    channels: [Channel; 4],
    overflow_events: [Rc<EventInfo>; 4],
}

impl Timers {
    pub fn new(system: &Shared<System>, irq: &Shared<Irq>) -> Self {
        Self {
            system: system.clone(),
            irq: irq.clone(),
            channels: Default::default(),
            overflow_events: Default::default(),
        }
    }

    pub fn reset(&mut self, arch: Arch) {
        match arch {
            Arch::ARMv4 => {
                self.overflow_events[0] = self.system.scheduler.register_event("Timer Overflow 7.0", |system| system.timer7.overflow(0));
                self.overflow_events[1] = self.system.scheduler.register_event("Timer Overflow 7.1", |system| system.timer7.overflow(1));
                self.overflow_events[2] = self.system.scheduler.register_event("Timer Overflow 7.2", |system| system.timer7.overflow(2));
                self.overflow_events[3] = self.system.scheduler.register_event("Timer Overflow 7.3", |system| system.timer7.overflow(3));
            }
            Arch::ARMv5 => {
                self.overflow_events[0] = self.system.scheduler.register_event("Timer Overflow 9.0", |system| system.timer9.overflow(0));
                self.overflow_events[1] = self.system.scheduler.register_event("Timer Overflow 9.1", |system| system.timer9.overflow(1));
                self.overflow_events[2] = self.system.scheduler.register_event("Timer Overflow 9.2", |system| system.timer9.overflow(2));
                self.overflow_events[3] = self.system.scheduler.register_event("Timer Overflow 9.3", |system| system.timer9.overflow(3));
            }
        }
    }

    pub fn read_length(&mut self, id: usize) -> u16 {
        self.update_counter(id)
    }

    pub const fn read_control(&self, id: usize) -> u16 {
        self.channels[id].control.0
    }

    pub fn write_length(&mut self, id: usize, val: u32, mask: u32) {
        self.channels[id].reload_value = (self.channels[id].reload_value & !mask) | (val & mask);
    }

    pub fn write_control(&mut self, id: usize, val: u16, mask: u32) {
        if self.channels[id].active {
            self.deactivate_channel(id)
        }

        let mask = (mask & 0xc7) as u16;
        let old_control = self.channels[id].control;
        self.channels[id].control.0 = (self.channels[id].control.0 & !mask) | (val & mask);
        self.channels[id].shift = SHIFTS[self.channels[id].control.prescaler()];

        if self.channels[id].control.start() {
            if !old_control.start() {
                self.channels[id].counter = self.channels[id].reload_value
            }

            if id == 0 || !self.channels[id].control.count_up() {
                self.activate_channel(id)
            }
        }
    }

    fn overflow(&mut self, id: usize) {
        self.channels[id].counter = self.channels[id].reload_value;

        if self.channels[id].control.irq() {
            self.irq.raise(IrqSource::timer(id));
        }

        if id == 0 || !self.channels[id].control.count_up() {
            self.activate_channel(id);
        }

        if id < 3 {
            if self.channels[id + 1].control.count_up() && self.channels[id + 1].control.start() {
                // in count up mode the next timer is incremented on overflow
                self.channels[id + 1].counter += 1;
                if self.channels[id + 1].counter == 0x10000 {
                    self.overflow(id + 1)
                }
            }
        }
    }

    fn activate_channel(&mut self, id: usize) {
        let channel = &mut self.channels[id];
        channel.active = true;
        channel.activation_timestamp = self.system.scheduler.get_current_time();

        let delay = (0x10000 - channel.counter as u64) << channel.shift;
        self.system.scheduler.add_event(delay, &self.overflow_events[id]);
    }

    fn deactivate_channel(&mut self, id: usize) {
        self.channels[id].counter = self.update_counter(id) as u32;
        self.channels[id].active = false;

        if self.channels[id].counter >= 0x10000 {
            error!("Timers: handle counter greater than 16-bits");
        }

        self.system.scheduler.cancel_event(&self.overflow_events[id]);
    }

    fn update_counter(&mut self, id: usize) -> u16 {
        let channel = &mut self.channels[id];
        if !channel.active {
            return channel.counter as u16
        }

        let delta = (self.system.scheduler.get_current_time() - channel.activation_timestamp) >> channel.shift;
        (channel.counter as u64 + delta) as u16
    }
}