use crate::arm::coprocessor::Coprocessor;
use crate::arm::cpu::{Arch, Cpu};
use crate::arm::memory::Memory;

use crate::util::Shared;

pub enum IrqSource {
    VBlank = 0,
    HBlank = 1,
    VCounter = 2,
    Timer0 = 3,
    Timer1 = 4,
    Timer2 = 5,
    Timer3 = 6,
    RTC = 7,
    DMA0 = 8,
    DMA1 = 9,
    DMA2 = 10,
    DMA3 = 11,
    Input = 12,
    IPCSync = 16,
    IPCSendEmpty = 17,
    IPCReceiveNonEmpty = 18,
    CartridgeTransfer = 19,
    GXFIFO = 21,
    SPI = 23,
}

// todo: replace cpu ref with Rc<Cell<bool>> or something
pub struct Irq {
    cpu: Shared<Cpu>,
    ime: bool,
    ie: u32,
    irf: u32,
}

impl Irq {
    pub fn new(cpu: &Shared<Cpu>) -> Self {
        Self {
            cpu: cpu.clone(),
            ime: false,
            ie: 0,
            irf: 0,
        }
    }

    pub fn reset(&mut self) {
        self.ime = false;
        self.ie = 0;
        self.irf = 0;
    }

    pub fn raise(&mut self, source: IrqSource) {
        let source = source as u32;

        self.irf |= 1 << source;
        if self.ie & (1 << source) != 0 {
            if self.ime || self.cpu.arch == Arch::ARMv4 {
                self.cpu.update_halted(false);
            }
        }
        self.update();
    }

    pub const fn read_ime(&self) -> bool {
        self.ime
    }
    pub const fn read_ie(&self) -> u32 {
        self.ie
    }
    pub const fn read_irf(&self) -> u32 {
        self.irf
    }

    pub fn write_ime(&mut self, val: u32, _mask: u32) {
        self.ime = val & 1 != 0;
        self.update();
    }

    pub fn write_ie(&mut self, val: u32, mask: u32) {
        self.ie = (self.ie & !mask) | (val & mask);
        self.update()
    }

    pub fn write_irf(&mut self, val: u32, mask: u32) {
        self.irf &= !(val & mask);
        self.update()
    }

    fn update(&mut self) {
        self.cpu.update_irq(self.ime && (self.ie & self.irf != 0))
    }
}
