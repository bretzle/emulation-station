struct Event {
    time: u64,
    kind: (),
}

#[derive(Default)]
pub struct Scheduler {
    current_time: u64,
    events: Vec<Event>,
}

impl Scheduler {
    pub fn get_current_time(&self) -> u64 {
        self.current_time
    }

    pub fn get_event_time(&self) -> u64 {
        // todo: there will always be an event... eventually...
        match self.events.get(0) {
            Some(e) => e.time,
            None => 16,
        }
    }

    pub fn tick(&mut self, cycles: u64) {
        self.current_time += cycles;
    }

    pub fn run(&mut self) {
        // todo
    }
}
