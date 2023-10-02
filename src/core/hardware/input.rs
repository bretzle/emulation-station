use crate::bitfield;

pub enum InputEvent {
    A,
    B,
    Start,
    Select,
    Right,
    Left,
    Up,
    Down,
    L,
    R,
}

#[derive(Copy, Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

bitfield! {
    struct KeyInput(u16) {
        a: bool => 0,
        b: bool => 1,
        select: bool => 2,
        start: bool => 3,
        right: bool => 4,
        left: bool => 5,
        up: bool => 6,
        down: bool => 7,
        r: bool => 8,
        l: bool => 9
    }
}

pub struct Input {
    point: Point,
    keyinput: KeyInput,
    extkeyin: u16,
}

impl Input {
    pub fn new() -> Self {
        Self {
            point: Point { x: 0, y: 0 },
            keyinput: KeyInput(0x3ff),
            extkeyin: 0x7f,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new()
    }

    pub fn handle_input(&mut self, event: InputEvent, pressed: bool) {
        match event {
            InputEvent::A => self.keyinput.set_a(!pressed),
            InputEvent::B => self.keyinput.set_b(!pressed),
            InputEvent::Start => self.keyinput.set_start(!pressed),
            InputEvent::Select => self.keyinput.set_select(!pressed),
            InputEvent::Left => self.keyinput.set_left(!pressed),
            InputEvent::Right => self.keyinput.set_right(!pressed),
            InputEvent::Up => self.keyinput.set_up(!pressed),
            InputEvent::Down => self.keyinput.set_down(!pressed),
            InputEvent::L => self.keyinput.set_l(!pressed),
            InputEvent::R => self.keyinput.set_r(!pressed),
        }
    }

    pub fn set_touch(&mut self, pressed: bool) {
        if pressed {
            self.extkeyin &= !(1 << 6)
        } else {
            self.extkeyin |= 1 << 6
        }
    }

    pub fn set_point(&mut self, x: u32, y: u32) {
        self.point.x = x;
        self.point.y = y;
    }

    pub fn touch_down(&self) -> bool {
        self.extkeyin & (1 << 6) == 0
    }

    pub fn get_point(&self) -> Point {
        self.point
    }

    pub fn read_keyinput(&self) -> u16 {
        self.keyinput.0
    }

    pub fn read_extkeyin(&self) -> u16 {
        self.extkeyin
    }
}
