use log::warn;
use std::any::Any;

use crate::arm::cpu::Arch;
use crate::arm::memory::{Memory, MmioMemory};
use crate::core::System;
use crate::util::*;

macro_rules! mmio {
    ($x:tt) => {
        $x >> 2
    };
}

const MMIO_DMA_SOURCE0: u32 = mmio!(0x040000b0);
const MMIO_DMA_DESTINATION0: u32 = mmio!(0x040000b4);
const MMIO_DMA_LENGTH0: u32 = mmio!(0x040000b8);
const MMIO_DMA_SOURCE1: u32 = mmio!(0x040000bc);
const MMIO_DMA_DESTINATION1: u32 = mmio!(0x040000c0);
const MMIO_DMA_LENGTH1: u32 = mmio!(0x040000c4);
const MMIO_DMA_SOURCE2: u32 = mmio!(0x040000c8);
const MMIO_DMA_DESTINATION2: u32 = mmio!(0x040000cc);
const MMIO_DMA_LENGTH2: u32 = mmio!(0x040000d0);
const MMIO_DMA_SOURCE3: u32 = mmio!(0x040000d4);
const MMIO_DMA_DESTINATION3: u32 = mmio!(0x040000d8);
const MMIO_DMA_LENGTH3: u32 = mmio!(0x040000dc);
const MMIO_RCNT: u32 = mmio!(0x04000134);
const MMIO_IPCSYNC: u32 = mmio!(0x04000180);
const MMIO_IPCFIFOCNT: u32 = mmio!(0x04000184);
const MMIO_IPCFIFOSEND: u32 = mmio!(0x04000188);
const MMIO_SPICNT: u32 = mmio!(0x040001c0);
const MMIO_IME: u32 = mmio!(0x04000208);
const MMIO_IE: u32 = mmio!(0x04000210);
const MMIO_IRF: u32 = mmio!(0x04000214);
const MMIO_VRAMSTAT: u32 = mmio!(0x04000240);
const MMIO_POSTFLG: u32 = mmio!(0x04000300);
const MMIO_POWCNT1: u32 = mmio!(0x04000304);
const MMIO_SPU_CHANNEL_BASE: u32 = mmio!(0x04000400);
const MMIO_SPU_CHANNEL_END: u32 = mmio!(0x040004fc);
const MMIO_SOUNDBIAS: u32 = mmio!(0x04000504);
const MMIO_IPCFIFORECV: u32 = mmio!(0x04100000);

pub struct Arm7Memory {
    system: Shared<System>,
    arm7_wram: Box<[u8]>,
    bios: Box<[u8]>,
    rcnt: u16,
    postflg: u8,
    pages: PageTable<14>,
}

impl Arm7Memory {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            arm7_wram: vec![0; 0x10000].into_boxed_slice(),
            bios: std::fs::read("firmware/bios7.bin").unwrap().into_boxed_slice(),
            rcnt: 0,
            postflg: 0,
            pages: PageTable::new(),
        }
    }

    pub fn update_wram_mapping(&mut self) {
        match self.system.wramcnt {
            0x0 => self.pages.map(
                0x03000000,
                0x03800000,
                self.arm7_wram.as_ptr() as _,
                0xffff,
                RegionAttributes::ReadWrite,
            ),
            0x1 => self.pages.map(
                0x03000000,
                0x03800000,
                self.system.shared_wram.as_ptr() as _,
                0x3fff,
                RegionAttributes::ReadWrite,
            ),
            0x2 => self.pages.map(
                0x03000000,
                0x03800000,
                unsafe { self.system.shared_wram.as_ptr().add(0x4000) as _ },
                0x3fff,
                RegionAttributes::ReadWrite,
            ),
            0x3 => self.pages.map(
                0x03000000,
                0x03800000,
                self.system.shared_wram.as_ptr() as _,
                0x7fff,
                RegionAttributes::ReadWrite,
            ),
            _ => unreachable!(),
        }

        self.pages.map(
            0x03800000,
            0x04000000,
            self.arm7_wram.as_ptr() as _,
            0xffff,
            RegionAttributes::ReadWrite,
        );
    }

    fn write_postflg(&mut self, val: u8) {
        self.postflg = val & 1
    }
}

impl Memory for Arm7Memory {
    fn reset(&mut self) {
        self.arm7_wram.fill(0);
        self.rcnt = 0;
        self.postflg = 0;

        let ptr = self.bios.as_mut_ptr();
        self.pages.map(0x00000000, 0x01000000, ptr, 0x3fff, RegionAttributes::Read);
        let ptr = self.system.main_memory.as_mut_ptr();
        self.pages.map(0x02000000, 0x03000000, ptr, 0x3fffff, RegionAttributes::ReadWrite);

        self.update_wram_mapping();
    }

    fn read_byte(&mut self, addr: u32) -> u8 {
        let ptr = self.pages.read_pointer::<u8>(addr);
        if !ptr.is_null() {
            return unsafe { std::ptr::read(ptr) };
        }

        match addr >> 24 {
            0x04 => self.mmio_read_byte(addr),
            0x06 => self.system.video_unit.vram.arm7_vram.read(addr),
            0x08 | 0x09 => todo!(),
            _ => {
                warn!("ARM7Memory: handle 8-bit read {addr:08x}");
                0
            }
        }
    }

    fn read_half(&mut self, addr: u32) -> u16 {
        let ptr = self.pages.read_pointer::<u16>(addr);
        if !ptr.is_null() {
            return unsafe { std::ptr::read(ptr) };
        }

        match addr >> 24 {
            0x04 => self.mmio_read_half(addr),
            0x06 => self.system.video_unit.vram.arm7_vram.read(addr),
            0x08 | 0x09 => todo!(),
            _ => {
                warn!("ARM7Memory: handle 16-bit read {addr:08x}");
                0
            }
        }
    }

    fn read_word(&mut self, addr: u32) -> u32 {
        let ptr = self.pages.read_pointer::<u32>(addr);
        if !ptr.is_null() {
            return unsafe { std::ptr::read(ptr) };
        }

        match addr >> 24 {
            0x04 => self.mmio_read_word(addr),
            0x06 => self.system.video_unit.vram.arm7_vram.read(addr),
            0x08 | 0x09 => todo!(),
            _ => {
                warn!("ARM7Memory: handle 32-bit read {addr:08x}");
                0
            }
        }
    }

    fn write_byte(&mut self, addr: u32, val: u8) {
        let ptr = self.pages.write_pointer::<u8>(addr);
        if !ptr.is_null() {
            return unsafe { std::ptr::write(ptr, val) };
        }

        match addr >> 24 {
            0x04 => self.mmio_write_byte(addr, val),
            0x06 => todo!(),
            _ => warn!("ARM7Memory: handle 8-bit write {addr:08x} = {val:02x}"),
        }
    }

    fn write_half(&mut self, addr: u32, val: u16) {
        let ptr = self.pages.write_pointer::<u16>(addr);
        if !ptr.is_null() {
            return unsafe { std::ptr::write(ptr, val) };
        }

        match addr >> 24 {
            0x04 => self.mmio_write_half(addr, val),
            0x06 => todo!(),
            _ => warn!("ARM7Memory: handle 16-bit write {addr:08x} = {val:04x}"),
        }
    }

    fn write_word(&mut self, addr: u32, val: u32) {
        let ptr = self.pages.write_pointer::<u32>(addr);
        if !ptr.is_null() {
            return unsafe { std::ptr::write(ptr, val) };
        }

        match addr >> 24 {
            0x04 => self.mmio_write_word(addr, val),
            0x06 => todo!(),
            0x08 | 0x09 => {}
            _ => warn!("ARM7Memory: handle 32-bit write {addr:08x} = {val:08x}"),
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl MmioMemory for Arm7Memory {
    fn mmio_read<const MASK: u32>(&mut self, addr: u32) -> u32 {
        let mut val = 0;
        match mmio!(addr) {
            MMIO_DMA_LENGTH3 => {
                if MASK & 0xffff != 0 {
                    val |= self.system.dma9.read_length(3)
                }
                if MASK & 0xffff0000 != 0 {
                    val |= (self.system.dma9.read_control(3) as u32) << 16
                }
            }
            MMIO_IPCSYNC => return self.system.ipc.read_ipcsync(Arch::ARMv4),
            MMIO_IPCFIFOCNT => return self.system.ipc.read_ipcfifocnt(Arch::ARMv4) as u32,
            MMIO_SPICNT => {
                if MASK & 0xffff != 0 {
                    val |= self.system.spi.read_spicnt() as u32;
                }
                if MASK & 0xffff0000 != 0 {
                    val |= (self.system.spi.read_spidata() as u32) << 16
                }
            }
            MMIO_IME => return self.system.arm7.get_irq().read_ime() as u32,
            MMIO_IE => return self.system.arm7.get_irq().read_ie(),
            MMIO_IRF => return self.system.arm7.get_irq().read_irf(),
            MMIO_VRAMSTAT => {
                if MASK & 0xff != 0 {
                    val |= self.system.video_unit.vram.read_vramstat() as u32
                }
                if MASK & 0xff00 != 0 {
                    val |= (self.system.read_wramcnt() as u32) << 8
                }
            }
            MMIO_POWCNT1 => return self.system.video_unit.read_powcnt1(),
            MMIO_IPCFIFORECV => return self.system.ipc.read_ipcfiforecv(Arch::ARMv4),
            _ => warn!(
                "ARM7Memory: unmapped {}-bit read {:08x}",
                get_access_size(MASK),
                addr + get_access_offset(MASK),
            ),
        }
        val
    }

    fn mmio_write<const MASK: u32>(&mut self, addr: u32, val: u32) {
        match mmio!(addr) {
            MMIO_DMA_SOURCE0 => self.system.dma7.write_source(0, val, MASK),
            MMIO_DMA_DESTINATION0 => self.system.dma7.write_destination(0, val, MASK),
            MMIO_DMA_LENGTH0 => {
                if MASK & 0xffff != 0 {
                    self.system.dma7.write_length(0, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma7.write_control(0, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMA_SOURCE1 => self.system.dma7.write_source(1, val, MASK),
            MMIO_DMA_DESTINATION1 => self.system.dma7.write_destination(1, val, MASK),
            MMIO_DMA_LENGTH1 => {
                if MASK & 0xffff != 0 {
                    self.system.dma7.write_length(1, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma7.write_control(1, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMA_SOURCE2 => self.system.dma7.write_source(2, val, MASK),
            MMIO_DMA_DESTINATION2 => self.system.dma7.write_destination(2, val, MASK),
            MMIO_DMA_LENGTH2 => {
                if MASK & 0xffff != 0 {
                    self.system.dma7.write_length(2, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma7.write_control(2, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMA_SOURCE3 => self.system.dma7.write_source(3, val, MASK),
            MMIO_DMA_DESTINATION3 => self.system.dma7.write_destination(3, val, MASK),
            MMIO_DMA_LENGTH3 => {
                if MASK & 0xffff != 0 {
                    self.system.dma7.write_length(3, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma7.write_control(3, val >> 16, MASK >> 16)
                }
            }
            MMIO_RCNT => {
                if MASK & 0xffff != 0 {
                    self.rcnt = val as _;
                }
            }
            MMIO_IPCSYNC => {
                if MASK & 0xffff != 0 {
                    self.system.ipc.write_ipcsync(Arch::ARMv4, val, MASK)
                }
            }
            MMIO_IPCFIFOCNT => {
                if MASK & 0xffff != 0 {
                    self.system.ipc.write_ipcfifocnt(Arch::ARMv4, val as _, MASK as _);
                }
            }
            MMIO_IPCFIFOSEND => self.system.ipc.write_ipcfifosend(Arch::ARMv4, val),
            MMIO_SPICNT => {
                if MASK & 0xffff != 0 {
                    self.system.spi.write_spicnt(val as _, MASK & 0xffff);
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.spi.write_spidata((val >> 16) as _)
                }
            }
            MMIO_IME => return self.system.arm7.get_irq().write_ime(val, MASK),
            MMIO_IE => return self.system.arm7.get_irq().write_ie(val, MASK),
            MMIO_IRF => return self.system.arm7.get_irq().write_irf(val, MASK),
            MMIO_POSTFLG => {
                if MASK & 0xff != 0 {
                    self.write_postflg(val as u8)
                }
                if MASK & 0xff00 != 0 {
                    todo!()
                }
            }
            MMIO_POWCNT1 => self.system.video_unit.write_powcnt1(val, MASK),
            MMIO_SPU_CHANNEL_BASE..=MMIO_SPU_CHANNEL_END => warn!("spu channel"),
            MMIO_SOUNDBIAS => warn!("todo: sound bias"),
            _ => warn!(
                "ARM7Memory: unmapped {}-bit write {:08x} = {:08x}",
                get_access_size(MASK),
                addr + get_access_offset(MASK),
                (val & MASK) >> (get_access_offset(MASK) * 8)
            ),
        }
    }
}
