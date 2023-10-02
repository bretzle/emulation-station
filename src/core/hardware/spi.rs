use log::debug;

use crate::arm::memory::Memory;
use crate::bitfield;
use crate::core::System;
use crate::util::Shared;

#[repr(u16)]
enum Device {
    Powerman = 0,
    Firmware = 1,
    Touchscreen = 2,
    Reserved = 3,
}

bitfield! {
    struct SpiCnt(u16) {
        baudrate: u16 => 0 | 1,
        // 2 | 6
        busy: bool => 7,
        device: u16 [Device] => 8 | 9,
        transfer_halfwords: bool => 10,
        chipselect_hold: bool => 11,
        // 12 | 13
        irq: bool => 14,
        enable: bool => 15
    }
}

pub struct Spi {
    system: Shared<System>,
    firmware: Box<[u8]>,

    spicnt: SpiCnt,
    spidata: u8,
    write_count: usize,
    write_enable_latch: bool,
    write_in_progress: bool,
    command: u8,
    address: u32,

    adc_x1: u16,
    adc_x2: u16,
    adc_y1: u16,
    adc_y2: u16,
    scr_x1: u8,
    scr_x2: u8,
    scr_y1: u8,
    scr_y2: u8,
    output: u16,
}

impl Spi {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            firmware: std::fs::read("firmware/firmware.bin").unwrap().into_boxed_slice(),
            spicnt: SpiCnt(0),
            spidata: 0,
            write_count: 0,
            write_enable_latch: false,
            write_in_progress: false,
            command: 0,
            address: 0,
            adc_x1: 0,
            adc_x2: 0,
            adc_y1: 0,
            adc_y2: 0,
            scr_x1: 0,
            scr_x2: 0,
            scr_y1: 0,
            scr_y2: 0,
            output: 0,
        }
    }

    pub fn reset(&mut self) {
        self.spicnt.0 = 0;
        self.spidata = 0;
        self.write_count = 0;
        self.write_enable_latch = false;
        self.write_in_progress = false;
        self.command = 0;
        self.address = 0;
        self.output = 0;

        self.load_calibration_points();
    }

    pub fn direct_boot(&mut self) {
        for i in 0..0x70 {
            self.system.arm9.get_memory().write_byte(
                0x027ffc80 + i,
                self.firmware[0x3ff00 + i as usize]
            )
        }
    }

    fn load_calibration_points(&mut self) {
        macro_rules! read {
            ($t:ty, $start:expr) => {
                <$t>::from_le_bytes(
                    self.firmware[$start..$start + std::mem::size_of::<$t>()]
                        .try_into()
                        .unwrap(),
                )
            };
        }

        let offset = read!(u16, 0x20) as usize * 8;

        self.adc_x1 = read!(u16, offset + 0x58);
        self.adc_y1 = read!(u16, offset + 0x5a);
        self.scr_x1 = read!(u8, offset + 0x5c);
        self.scr_y1 = read!(u8, offset + 0x5d);
        self.adc_x2 = read!(u16, offset + 0x5e);
        self.adc_y2 = read!(u16, offset + 0x60);
        self.scr_x2 = read!(u8, offset + 0x62);
        self.scr_y2 = read!(u8, offset + 0x63);

        debug!("SPI: touchscreen calibration points loaded successfully")
    }

    pub const fn read_spicnt(&self) -> u16 {
        self.spicnt.0
    }

    pub const fn read_spidata(&self) -> u8 {
        self.spidata
    }

    pub fn write_spicnt(&mut self, val: u16, mask: u32) {
        let mask = (mask & 0xcf03) as u16;
        self.spicnt.0 = (self.spicnt.0 & !mask) | (val & mask);
    }

    pub fn write_spidata(&mut self, val: u8) {
        if self.spicnt.enable() {
            self.transfer(val)
        } else {
            self.spidata = 0
        }
    }

    fn transfer(&mut self, val: u8) {
        if self.write_count == 0 {
            self.command = val;
            self.address = 0;
            self.spidata = 0;
        } else {
            match self.spicnt.device() {
                Device::Powerman => todo!(),
                Device::Firmware => todo!(),
                Device::Touchscreen => todo!(),
                Device::Reserved => todo!(),
            }
        }
    }
}
