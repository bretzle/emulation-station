use crate::arm::memory::Memory;

pub struct Arm9Memory {}

impl Arm9Memory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Memory for Arm9Memory {
    fn read_byte(&mut self, addr: u32) -> u8 {
        todo!()
    }

    fn read_half(&mut self, addr: u32) -> u16 {
        todo!()
    }

    fn read_word(&mut self, addr: u32) -> u32 {
        todo!()
    }

    fn write_byte(&mut self, addr: u32, val: u8) {
        todo!()
    }

    fn write_half(&mut self, addr: u32, val: u16) {
        todo!()
    }

    fn write_word(&mut self, addr: u32, val: u32) {
        todo!()
    }
}
