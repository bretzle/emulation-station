use std::time::{Duration, Instant};

pub struct FrameCounter {
    begin: Instant,
    count: u32,
    queue_reset: bool,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self {
            begin: Instant::now(),
            count: 0,
            queue_reset: true,
        }
    }

    pub fn reset(&mut self) {
        self.begin = Instant::now();
        self.count = 0;
        self.queue_reset = false;
    }

    pub fn queue_reset(&mut self) {
        self.queue_reset = true;
    }

    pub fn fps(&mut self) -> Option<f64> {
        if self.queue_reset {
            self.reset();
            return None;
        }

        let delta = Instant::now() - self.begin;
        if delta < Duration::from_secs(1) {
            return None;
        }

        let fps = self.count as f64 / delta.as_secs_f64();
        self.reset();
        Some(fps)
    }

    pub fn inc(&mut self) -> &mut Self {
        self.count += 1;
        self
    }
}
