pub enum Bus {
    Code,
    Data,
    System,
}

#[repr(u8)]
pub enum RegionAttributes {
    Read = 0b01,
    Write = 0b10,
    ReadWrite = 0b11,
}

pub trait Memory {
    fn read_byte(&mut self, addr: u32) -> u8;
    fn read_half(&mut self, addr: u32) -> u16;
    fn read_word(&mut self, addr: u32) -> u32;

    fn write_byte(&mut self, addr: u32, val: u8);
    fn write_half(&mut self, addr: u32, val: u16);
    fn write_word(&mut self, addr: u32, val: u32);
}
