use std::time::{Duration, Instant};

const REFRESH_RATE: f64 = 60.0;

pub struct FrameHelper {
    accumulated: Duration,
    frame_delta: Duration,
    next: Instant,
    fast_forward: f64,
    begin: Instant,
    fps_count: u32,
    update_count: u32,
    queue_reset: bool,
}

impl FrameHelper {
    pub fn new() -> Self {
        let mut lim = Self {
            accumulated: Duration::ZERO,
            frame_delta: Duration::ZERO,
            next: Instant::now(),
            fast_forward: 1.0,
            begin: Instant::now(),
            fps_count: 0,
            update_count: 0,
            queue_reset: false,
        };
        lim.set_fps(REFRESH_RATE);
        lim
    }

    pub fn reset(&mut self) {
        self.accumulated = Duration::ZERO;
        self.queue_reset = false;
    }

    pub fn reset_counter(&mut self) {
        self.begin = Instant::now();
        self.fps_count = 0;
        self.update_count = 0;
    }

    pub fn queue_reset(&mut self) {
        self.queue_reset = true;
    }

    // pub const fn get_fast_forward(&self) -> f64 {
    //     self.fast_forward
    // }

    pub fn set_fast_forward(&mut self, val: f64) {
        self.fast_forward = val;
        self.set_fps(REFRESH_RATE * val);
    }

    pub fn run<F: FnOnce()>(&mut self, frame: F) {
        if self.next <= Instant::now() {
            self.next = Instant::now() + self.frame_delta;
            self.update_count += 1;
            frame();
        }

        if self.queue_reset {
            self.reset()
        }
    }

    fn set_fps(&mut self, fps: f64) {
        self.frame_delta = Duration::from_secs_f64(1.0 / fps);
        self.queue_reset();
    }

    pub fn fps(&mut self) -> Option<(f32, f32)> {
        if self.queue_reset {
            self.reset_counter();
            return None;
        }

        let delta = Instant::now() - self.begin;
        if delta < Duration::from_secs(1) {
            return None;
        }

        let fps = self.fps_count as f32 / delta.as_secs_f32();
        let ups = self.update_count as f32 / delta.as_secs_f32();
        self.reset_counter();
        Some((fps.round(), ups.round()))
    }

    pub fn inc(&mut self) -> &mut Self {
        self.fps_count += 1;
        self
    }
}
