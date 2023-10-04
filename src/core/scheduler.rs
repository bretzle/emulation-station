use std::rc::Rc;
use log::trace;

use crate::core::System;
use crate::util::Shared;

struct Event {
    time: u64,
    info: Rc<EventInfo>,
}

pub struct EventInfo {
    name: String,
    id: usize,
    callback: fn(&mut System),
}

impl Default for EventInfo {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            id: usize::MAX,
            callback: |_| unreachable!(),
        }
    }
}

pub struct Scheduler {
    system: Shared<System>,
    events: Vec<Event>,
    current_time: u64,
    current_event_id: usize,
}

impl Scheduler {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            events: vec![],
            current_time: 0,
            current_event_id: 0,
        }
    }

    pub fn reset(&mut self) {
        self.events.clear();
        self.current_time = 0;
        self.current_event_id = 0;
    }

    pub fn tick(&mut self, cycles: u64) {
        self.current_time += cycles;
    }

    pub fn run(&mut self) {
        let mut to_remove = vec![];
        for (idx, event) in self.events.iter().enumerate() {
            if event.time <= self.current_time {
                // trace!("running '{}' at {}", event.info.name, event.time);
                to_remove.push(idx);
                (event.info.callback)(&mut self.system);
            }
        }

        for idx in to_remove.into_iter().rev() {
            self.events.remove(idx);
        }
    }

    pub fn add_event(&mut self, delay: u64, info: &Rc<EventInfo>) {
        let time = self.current_time + delay;
        let event = Event { time, info: info.clone() };
        let index = self.calc_event_index(&event);
        self.events.insert(index, event);
    }

    pub fn cancel_event(&mut self, info: &EventInfo) {
        self.events.retain(|e| e.info.id != info.id);
    }

    pub fn register_event(&mut self, name: &str, callback: fn(&mut System)) -> Rc<EventInfo> {
        let info = EventInfo {
            name: name.to_string(),
            id: self.current_event_id,
            callback,
        };
        self.current_event_id += 1;
        Rc::new(info)
    }

    pub fn get_current_time(&self) -> u64 {
        self.current_time
    }

    pub fn get_event_time(&self) -> u64 {
        assert!(!self.events.is_empty());
        self.events.get(0).map(|e| e.time).unwrap_or(u64::MAX)
    }

    fn calc_event_index(&self, event: &Event) -> usize {
        match self.events.binary_search_by(|other| other.time.cmp(&event.time)) {
            Ok(idx) => idx,
            Err(idx) => idx,
        }
    }
}
