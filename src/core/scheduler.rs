use std::rc::Rc;

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
        for event in &self.events {
            if event.time <= self.current_time {
                // trace!("running '{}' at {}", event.info.name, event.time);
                (event.info.callback)(&mut self.system);
            }
        }

        self.events.retain(|e| e.time > self.current_time);
    }

    pub fn add_event(&mut self, delay: u64, info: &Rc<EventInfo>) {
        let time = self.current_time + delay;
        let event = Event {
            time,
            info: info.clone(),
        };
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
        if self.events.len() == 0 {
            panic!()
        }
        self.events.get(0).map(|e| e.time).unwrap_or(u64::MAX)
    }

    // todo: replace with Vec::binary_search_by
    fn calc_event_index(&self, event: &Event) -> usize {
        if self.events.is_empty() {
            return 0;
        }

        let mut lower = 0;
        let mut upper = self.events.len() - 1;

        while lower <= upper {
            let mid = (lower + upper) / 2;
            if event.time > self.events[mid].time {
                lower = mid + 1;
            } else {
                upper = mid - 1;
            }
        }

        lower
    }
}
