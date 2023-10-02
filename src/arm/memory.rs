use std::any::Any;

// pub enum Bus {
//     Code,
//     Data,
//     System,
// }

pub trait Memory {
    fn reset(&mut self);

    fn read_byte(&mut self, addr: u32) -> u8;
    fn read_half(&mut self, addr: u32) -> u16;
    fn read_word(&mut self, addr: u32) -> u32;

    fn write_byte(&mut self, addr: u32, val: u8);
    fn write_half(&mut self, addr: u32, val: u16);
    fn write_word(&mut self, addr: u32, val: u32);

    fn as_any(&mut self) -> &mut dyn Any;
}

/// this really shouldn't be a trait, but is an easy way to prevent duplicate code
pub trait MmioMemory {
    fn mmio_read_byte(&mut self, addr: u32) -> u8 {
        match addr & 0x3 {
            0 => (self.mmio_read::<0x000000ff>(addr & !0x3) >> 0) as u8,
            1 => (self.mmio_read::<0x0000ff00>(addr & !0x3) >> 8) as u8,
            2 => (self.mmio_read::<0x00ff0000>(addr & !0x3) >> 16) as u8,
            3 => (self.mmio_read::<0xff000000>(addr & !0x3) >> 24) as u8,
            _ => unreachable!(),
        }
    }

    fn mmio_read_half(&mut self, addr: u32) -> u16 {
        match addr & 0x2 {
            0 => (self.mmio_read::<0x0000ffff>(addr & !0x2) >> 0) as u16,
            2 => (self.mmio_read::<0xffff0000>(addr & !0x2) >> 16) as u16,
            _ => unreachable!(),
        }
    }

    fn mmio_read_word(&mut self, addr: u32) -> u32 {
        self.mmio_read::<0xffffffff>(addr)
    }

    fn mmio_read<const MASK: u32>(&mut self, addr: u32) -> u32;

    fn mmio_write_byte(&mut self, addr: u32, val: u8) {
        let mirrored = val as u32 * 0x01010101;
        match addr & 0x3 {
            0x0 => self.mmio_write::<0x000000ff>(addr & !0x3, mirrored),
            0x1 => self.mmio_write::<0x0000ff00>(addr & !0x3, mirrored),
            0x2 => self.mmio_write::<0x00ff0000>(addr & !0x3, mirrored),
            0x3 => self.mmio_write::<0xff000000>(addr & !0x3, mirrored),
            _ => unreachable!(),
        }
    }

    fn mmio_write_half(&mut self, addr: u32, val: u16) {
        let mirrored = val as u32 * 0x00010001;
        match addr & 0x2 {
            0x0 => self.mmio_write::<0x0000ffff>(addr & !0x2, mirrored),
            0x2 => self.mmio_write::<0xffff0000>(addr & !0x2, mirrored),
            _ => unreachable!(),
        }
    }

    fn mmio_write_word(&mut self, addr: u32, val: u32) {
        self.mmio_write::<0xffffffff>(addr, val)
    }

    fn mmio_write<const MASK: u32>(&mut self, addr: u32, val: u32);
}
