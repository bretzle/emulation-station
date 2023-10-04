use crate::bitfield;

enum SampleOutput {
    Mixer = 0,
    Channel1 = 1,
    Channel2 = 2,
    Channel1And3 = 3,
}

bitfield! {
    struct SoundCnt(u16) {
        master_volume: u16 => 0 | 6,
        // 7
        left_output: u8 [SampleOutput] => 8 | 9,
        right_output: u8 [SampleOutput] => 10 | 11,
        skip_ch1_mixer_output: bool => 12,
        skip_ch3_mixer_output: bool => 13,
        // 14
        master_enable: bool => 15
    }
}

pub struct Spu {
    soundcnt: SoundCnt
}

impl Spu {
    pub fn new() -> Self {
        Self {
            soundcnt: SoundCnt(0)
        }
    }

    pub fn reset(&mut self) {
        // todo
    }

    pub const fn read_soundcnt(&self) -> u16 {
        self.soundcnt.0
    }

    pub fn write_soundcnt(&mut self, val: u16, mask: u16) {
        self.soundcnt.0 = (self.soundcnt.0 & !mask) | (val & mask)
    }
}