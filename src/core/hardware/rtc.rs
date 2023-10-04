use crate::bitfield;

bitfield! {
    #[derive(Clone, Copy)]
    struct Register(u8) {
        data_io: bool => 0,
        clock: bool => 1,
        select: bool => 2,
        // 3
        data_io_direction: bool => 4,
        clock_direction: bool => 5,
        select_direction: bool => 6
        // 7
    }
}

pub struct Rtc {
    rtc: Register,
    write_count: u8,
    command: u8,
    status1: u8,
    status2: u8,
    date_time: [u8; 7],
}

impl Rtc {
    pub const fn new() -> Self {
        Self {
            rtc: Register(0),
            write_count: 0,
            command: 0,
            status1: 0,
            status2: 0,
            date_time: [0; 7],
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new()
    }

    pub const fn read_rtc(&self) -> u8 {
        self.rtc.0
    }

    pub fn write_rtc(&mut self, val: u8) {
        let old_rtc = self.rtc;
        self.rtc.0 = val;

        if !old_rtc.select() && self.rtc.select() {
            if old_rtc.clock() && !self.rtc.clock() {
                if self.write_count < 8 {
                    self.command |= (self.rtc.0 & 0x1) << self.write_count;
                } else if self.rtc.data_io_direction() {
                    self.interpret_write_command(self.rtc.0);
                } else {
                    self.rtc.0 = (self.rtc.0 & !0x1) | (old_rtc.0 & 0x1);
                    self.rtc.0 = self.interpret_read_command(self.rtc.0);
                }

                self.write_count += 1;
            } else if !self.rtc.data_io_direction() {
                self.rtc.0 = (self.rtc.0 & !0x1) | (old_rtc.0 & 0x1);
            }
        } else {
            self.write_count = 0;
            self.command = 0;
        }
    }

    fn interpret_read_command(&mut self, val: u8) -> u8 {
        todo!()
    }

    fn interpret_write_command(&mut self, val: u8) {
        todo!()
    }

    const fn convert_bcd(val: u8) -> u8 {
        ((val / 10) << 4) | (val % 10)
    }
}