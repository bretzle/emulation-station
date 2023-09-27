use std::time::{Duration, Instant};

const REFRESH_RATE: f64 = 59.737;

#[derive(Default)]
pub struct FrameLimiter {
    accumulated: Duration,
    frame_delta: Duration,
    fast_forward: f64,
    queue_reset: bool,
}

impl FrameLimiter {
    pub fn new() -> Self {
        let mut lim = Self {
            accumulated: Duration::ZERO,
            frame_delta: Duration::ZERO,
            fast_forward: 1.0,
            queue_reset: false,
        };
        lim.set_fps(REFRESH_RATE);
        lim
    }

    pub fn reset(&mut self) {
        self.accumulated = Duration::ZERO;
        self.queue_reset = false;
    }

    pub fn queue_reset(&mut self) {
        self.queue_reset = true;
    }

    pub fn is_fast_forward(&self) -> bool {
        self.fast_forward > 1.0
    }

    pub fn set_fast_forward(&mut self, val: f64) {
        self.fast_forward = val;
        self.set_fps(REFRESH_RATE * val);
    }

    pub fn run<F: FnOnce()>(&mut self, frame: F) {
        macro_rules! measure {
            ($t:expr) => {{
                let now = Instant::now();
                $t;
                now.elapsed()
            }};
        }

        self.accumulated += measure!(frame());

        if self.accumulated < self.frame_delta {
            self.accumulated += measure!(std::thread::sleep(self.frame_delta - self.accumulated));
        }
        self.accumulated -= self.frame_delta;

        if self.queue_reset {
            self.reset()
        }
    }

    fn set_fps(&mut self, fps: f64) {
        self.frame_delta = Duration::from_secs_f64(1.0 / fps);
        self.queue_reset();
    }
}
