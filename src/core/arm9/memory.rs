use log::{error, warn};
use std::any::Any;

use crate::arm::coprocessor::Tcm;
use crate::arm::cpu::Arch;
use crate::arm::memory::{Memory, PageTable, RegionAttributes};
use crate::core::video::vram::VramBank;
use crate::core::System;
use crate::util::Shared;

macro_rules! mmio {
    ($x:tt) => {
        $x >> 2
    };
}

pub struct Arm9Memory {
    system: Shared<System>,
    postflg: u8,
    bios: Box<[u8]>,
    dtcm_data: Box<[u8]>,
    itcm_data: Box<[u8]>,

    pub itcm: Shared<Tcm>,
    pub dtcm: Shared<Tcm>,

    read_table: PageTable<14>,
    write_table: PageTable<14>,
}

impl Arm9Memory {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            postflg: 0,
            bios: std::fs::read("firmware/bios9.bin").unwrap().into_boxed_slice(),
            dtcm_data: vec![0; 0x4000].into_boxed_slice(),
            itcm_data: vec![0; 0x8000].into_boxed_slice(),

            itcm: Shared::default(),
            dtcm: Shared::default(),

            read_table: PageTable::new(),
            write_table: PageTable::new(),
        }
    }

    pub fn update_wram_mapping(&mut self) {
        unsafe {
            match self.system.wramcnt {
                0x0 => {
                    self.map(
                        0x03000000,
                        0x04000000,
                        self.system.shared_wram.as_ptr() as _,
                        0x7fff,
                        RegionAttributes::ReadWrite,
                    );
                }
                0x1 => {
                    self.map(
                        0x03000000,
                        0x04000000,
                        self.system.shared_wram.as_ptr().add(0x4000) as _,
                        0x3fff,
                        RegionAttributes::ReadWrite,
                    );
                }
                0x2 => {
                    self.map(
                        0x03000000,
                        0x04000000,
                        self.system.shared_wram.as_ptr() as _,
                        0x3fff,
                        RegionAttributes::ReadWrite,
                    );
                }
                0x3 => {
                    self.unmap(0x03000000, 0x04000000, RegionAttributes::ReadWrite);
                }
                _ => unreachable!(),
            }
        }
    }

    unsafe fn map(&mut self, base: u32, end: u32, ptr: *mut u8, mask: u32, attributes: RegionAttributes) {
        match attributes {
            RegionAttributes::Read => {
                self.read_table.map(base, end, ptr, mask);
            }
            RegionAttributes::Write => {
                self.write_table.map(base, end, ptr, mask);
            }
            RegionAttributes::ReadWrite => {
                self.read_table.map(base, end, ptr, mask);
                self.write_table.map(base, end, ptr, mask);
            }
        }
    }

    unsafe fn unmap(&mut self, base: u32, end: u32, attributes: RegionAttributes) {
        match attributes {
            RegionAttributes::Read => self.read_table.unmap(base, end),
            RegionAttributes::Write => self.write_table.unmap(base, end),
            RegionAttributes::ReadWrite => {
                self.read_table.unmap(base, end);
                self.write_table.unmap(base, end);
            }
        }
    }

    fn tcm_write<T>(&mut self, addr: u32, val: T) -> bool {
        let Self { itcm, dtcm, .. } = self;

        // TODO: if bus != system
        if itcm.enable_writes && addr >= itcm.base && addr < itcm.limit {
            // common::write<T>(itcm.data, value, (addr - itcm.config.base) & itcm.mask);
            // return;
            let ptr = itcm.data;
            let val = val;
            let offset = (addr - itcm.base) & itcm.mask;
            unsafe { *ptr.add(offset as usize).cast() = val };
            return true;
        }

        // TODO: if bus = Data
        if dtcm.enable_writes && addr >= dtcm.base && addr < dtcm.limit {
            let ptr = dtcm.data;
            let val = val;
            let offset = (addr - dtcm.base) & dtcm.mask;
            unsafe { *ptr.add(offset as usize).cast() = val };
            // common::write<T>(dtcm.data, value, (addr - dtcm.config.base) & dtcm.mask);
            return true;
        }

        let ptr = self.write_table.get_pointer::<T>(addr);
        if !ptr.is_null() {
            unsafe { std::ptr::write(ptr, val) }
            return true;
        }

        false
    }

    fn tcm_read<T: Copy>(&mut self, addr: u32) -> Option<T> {
        let Self { itcm, dtcm, .. } = self;

        // TODO: if bus != System
        if itcm.enable_reads && addr >= itcm.base && addr < itcm.limit {
            // return common::read<T>(itcm.data, (addr - itcm.config.base) & itcm.mask);
            return Some(unsafe {
                let offset = (addr - itcm.base) & itcm.mask;
                *itcm.data.add(offset as usize).cast::<T>()
            });
        }

        // TODO: if bus = Data
        if dtcm.enable_reads && addr >= dtcm.base && addr < dtcm.limit {
            // return common::read<T>(dtcm.data, (addr - dtcm.config.base) & dtcm.mask);
            return Some(unsafe {
                let offset = (addr - dtcm.base) & dtcm.mask;
                *dtcm.data.add(offset as usize).cast::<T>()
            });
        }

        let ptr = self.read_table.get_pointer::<T>(addr);
        if !ptr.is_null() {
            return Some(unsafe { std::ptr::read(ptr) });
        }

        None
    }

    fn write_postflg(&mut self, val: u8) {
        self.postflg = (self.postflg & !0x2) | (val & 0x3)
    }
}

fn get_access_size(mut mask: u32) -> u32 {
    let mut size = 0;
    for _ in 0..4 {
        if mask & 0xff != 0 {
            size += 8;
        }
        mask >>= 8;
    }
    size
}

fn get_access_offset(mut mask: u32) -> u32 {
    let mut offset = 0;
    for _ in 0..4 {
        if mask & 0xff != 0 {
            break;
        }
        offset += 1;
        mask >>= 8;
    }
    offset
}

impl Memory for Arm9Memory {
    fn reset(&mut self) {
        self.postflg = 0;
        self.dtcm_data.fill(0);
        self.itcm_data.fill(0);

        self.dtcm.data = self.dtcm_data.as_mut_ptr();
        self.itcm.data = self.itcm_data.as_mut_ptr();
        self.dtcm.mask = self.dtcm_data.len() as u32 - 1;
        self.itcm.mask = self.itcm_data.len() as u32 - 1;

        unsafe {
            let ptr = self.bios.as_mut_ptr();
            self.map(0xffff0000, 0xffff8000, ptr, 0x7fff, RegionAttributes::Read);
            let ptr = self.system.main_memory.as_mut_ptr();
            self.map(0x02000000, 0x03000000, ptr, 0x3fffff, RegionAttributes::ReadWrite);
        }
        self.update_wram_mapping();
    }

    fn read_byte(&mut self, addr: u32) -> u8 {
        if let Some(val) = self.tcm_read::<u8>(addr) {
            return val;
        }

        match addr >> 24 {
            0x04 => self.mmio_read_byte(addr),
            0x05 => todo!(),
            0x06 => todo!(),
            0x07 => todo!(),
            0x08 | 0x09 => todo!(),
            _ => {
                warn!("ARM9Memory: handle 8-bit read {addr:08x}");
                0
            }
        }
    }

    fn read_half(&mut self, addr: u32) -> u16 {
        let addr = addr & !1;
        if let Some(val) = self.tcm_read::<u16>(addr) {
            return val;
        }

        match addr >> 24 {
            0x04 => self.mmio_read_half(addr),
            0x05 => todo!(),
            0x06 => todo!(),
            0x07 => todo!(),
            0x08 | 0x09 => todo!(),
            _ => {
                warn!("ARM9Memory: handle 16-bit read {addr:08x}");
                0
            }
        }
    }

    fn read_word(&mut self, addr: u32) -> u32 {
        let addr = addr & !3;
        if let Some(val) = self.tcm_read::<u32>(addr) {
            return val;
        }

        match addr >> 24 {
            0x00 | 0x01 => 0,
            0x04 => self.mmio_read_word(addr),
            0x05 => todo!(),
            0x06 => self.system.video_unit.vram.read(addr),
            0x07 => todo!(),
            0x08 | 0x09 => todo!(),
            0x0a => todo!(),
            _ => {
                warn!("ARM9Memory: handle 32-bit read {addr:08x}");
                0
            }
        }
    }

    fn write_byte(&mut self, addr: u32, val: u8) {
        if self.tcm_write(addr, val) {
            return;
        }

        match addr >> 24 {
            0x04 => self.mmio_write_byte(addr, val),
            0x06 => todo!(),
            _ => warn!("ARM9Memory: handle 8-bit write {addr:08x} = {val:02x}"),
        }
    }

    fn write_half(&mut self, addr: u32, val: u16) {
        let addr = addr & !1;
        if self.tcm_write(addr, val) {
            return;
        }
        match addr >> 24 {
            0x04 => self.mmio_write_half(addr, val),
            0x05 => todo!(),
            0x06 => self.system.video_unit.vram.write(addr, val),
            0x07 => todo!(),
            _ => warn!("ARM9Memory: handle 16-bit write {addr:08x} = {val:04x}"),
        }
    }

    fn write_word(&mut self, addr: u32, val: u32) {
        let addr = addr & !3;
        if self.tcm_write(addr, val) {
            return;
        }
        match addr >> 24 {
            0x00 | 0x01 => {}
            0x04 => self.mmio_write_word(addr, val),
            0x05 => self.system.video_unit.write_palette_ram(addr, val),
            0x06 => self.system.video_unit.vram.write(addr, val),
            0x07 => self.system.video_unit.write_oam(addr, val),
            0x08 | 0x09 => {} // ignore gpa cart writes
            _ => warn!("ARM9Memory: handle 32-bit write {addr:08x} = {val:08x}"),
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

const MMIO_DISPCNT: u32 = mmio!(0x04000000);
const MMIO_DISPSTAT: u32 = mmio!(0x04000004);
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
const MMIO_DMAFILL_BASE: u32 = mmio!(0x040000e0);
const MMIO_DMAFILL_END: u32 = mmio!(0x040000ec);
const MMIO_KEYINPUT: u32 = mmio!(0x04000130);
const MMIO_IPCSYNC: u32 = mmio!(0x04000180);
const MMIO_IPCFIFOCNT: u32 = mmio!(0x04000184);
const MMIO_IPCFIFOSEND: u32 = mmio!(0x04000188);
const MMIO_IME: u32 = mmio!(0x04000208);
const MMIO_IE: u32 = mmio!(0x04000210);
const MMIO_IRF: u32 = mmio!(0x04000214);
const MMIO_VRAMCNT: u32 = mmio!(0x04000240);
const MMIO_VRAMCNT2: u32 = mmio!(0x04000244);
const MMIO_VRAMCNT3: u32 = mmio!(0x04000248);
const MMIO_DIVCNT: u32 = mmio!(0x04000280);
const MMIO_DIV_NUMER: u32 = mmio!(0x04000290);
const MMIO_DIV_NUMER2: u32 = mmio!(0x04000294);
const MMIO_DIV_DENOM: u32 = mmio!(0x04000298);
const MMIO_DIV_DENOM2: u32 = mmio!(0x0400029c);
const MMIO_DIV_RESULT: u32 = mmio!(0x040002a0);
const MMIO_DIV_RESULT2: u32 = mmio!(0x040002a4);
const MMIO_DIV_REM_RESULT: u32 = mmio!(0x040002a8);
const MMIO_DIV_REM_RESULT2: u32 = mmio!(0x040002ac);
const MMIO_SQRT_CNT: u32 = mmio!(0x040002b0);
const MMIO_SQRT_RESULT: u32 = mmio!(0x040002b4);
const MMIO_SQRT_PARAM: u32 = mmio!(0x040002b8);
const MMIO_SQRT_PARAM2: u32 = mmio!(0x040002bc);
const MMIO_POSTFLG: u32 = mmio!(0x04000300);
const MMIO_POWCNT1: u32 = mmio!(0x04000304);
const MMIO_IPCFIFORECV: u32 = mmio!(0x04100000);

impl Arm9Memory {
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

    fn mmio_write<const MASK: u32>(&mut self, addr: u32, val: u32) {
        match mmio!(addr) {
            MMIO_DISPCNT => self.system.video_unit.ppu_a.write_dispcnt(val, MASK),
            MMIO_DMA_SOURCE0 => self.system.dma9.write_source(0, val, MASK),
            MMIO_DMA_DESTINATION0 => self.system.dma9.write_destination(0, val, MASK),
            MMIO_DMA_LENGTH0 => {
                if MASK & 0xffff != 0 {
                    self.system.dma9.write_length(0, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma9.write_control(0, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMA_SOURCE1 => self.system.dma9.write_source(1, val, MASK),
            MMIO_DMA_DESTINATION1 => self.system.dma9.write_destination(1, val, MASK),
            MMIO_DMA_LENGTH1 => {
                if MASK & 0xffff != 0 {
                    self.system.dma9.write_length(1, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma9.write_control(1, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMA_SOURCE2 => self.system.dma9.write_source(2, val, MASK),
            MMIO_DMA_DESTINATION2 => self.system.dma9.write_destination(2, val, MASK),
            MMIO_DMA_LENGTH2 => {
                if MASK & 0xffff != 0 {
                    self.system.dma9.write_length(2, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma9.write_control(2, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMA_SOURCE3 => self.system.dma9.write_source(3, val, MASK),
            MMIO_DMA_DESTINATION3 => self.system.dma9.write_destination(3, val, MASK),
            MMIO_DMA_LENGTH3 => {
                if MASK & 0xffff != 0 {
                    self.system.dma9.write_length(3, val, MASK)
                }
                if MASK & 0xffff0000 != 0 {
                    self.system.dma9.write_control(3, val >> 16, MASK >> 16)
                }
            }
            MMIO_DMAFILL_BASE..=MMIO_DMAFILL_END => self.system.dma9.write_dmafill(addr, val),
            MMIO_IPCSYNC => {
                if MASK & 0xffff != 0 {
                    self.system.ipc.write_ipcsync(Arch::ARMv5, val, MASK);
                }
            }
            MMIO_IPCFIFOCNT => {
                if MASK & 0xffff != 0 {
                    self.system.ipc.write_ipcfifocnt(Arch::ARMv5, val as _, MASK as _);
                }
            }
            MMIO_IPCFIFOSEND => self.system.ipc.write_ipcfifosend(Arch::ARMv5, val),
            MMIO_IME => self.system.arm9.get_irq().write_ime(val, MASK),
            MMIO_IE => self.system.arm9.get_irq().write_ie(val, MASK),
            MMIO_IRF => self.system.arm9.get_irq().write_irf(val, MASK),
            MMIO_VRAMCNT => {
                if MASK & 0xff != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::A, val as u8)
                }
                if MASK & 0xff00 != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::B, (val >> 8) as u8)
                }
                if MASK & 0xff0000 != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::C, (val >> 16) as u8)
                }
                if MASK & 0xff000000 != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::D, (val >> 24) as u8)
                }
            }
            MMIO_VRAMCNT2 => {
                if MASK & 0xff != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::E, val as u8)
                }
                if MASK & 0xff00 != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::F, (val >> 8) as u8)
                }
                if MASK & 0xff0000 != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::G, (val >> 16) as u8)
                }
                if MASK & 0xff000000 != 0 {
                    self.system.write_wramcnt((val >> 24) as u8)
                }
            }
            MMIO_VRAMCNT3 => {
                if MASK & 0xff != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::H, val as u8)
                }
                if MASK & 0xff00 != 0 {
                    self.system.video_unit.vram.write_vramcnt(VramBank::I, (val >> 8) as u8)
                }
            }
            MMIO_DIVCNT => self.system.math_unit.write_divcnt(val as _, MASK as _),
            MMIO_DIV_NUMER => self.system.math_unit.write_div_numer(val as _, MASK as _),
            MMIO_DIV_NUMER2 => self.system.math_unit.write_div_numer((val as u64) << 32, (MASK as u64) << 32),
            MMIO_DIV_DENOM => self.system.math_unit.write_div_denom(val as _, MASK as _),
            MMIO_DIV_DENOM2 => self.system.math_unit.write_div_denom((val as u64) << 32, (MASK as u64) << 32),
            // MMIO_DIV_RESULT => unreachable!(),
            // MMIO_DIV_RESULT2 => unreachable!(),
            // MMIO_DIV_REM_RESULT => unreachable!(),
            // MMIO_DIV_REM_RESULT2 => unreachable!(),
            MMIO_SQRT_CNT => self.system.math_unit.write_sqrtcnt(val as _, MASK as _),
            // MMIO_SQRT_RESULT => unreachable!(),
            MMIO_SQRT_PARAM => self.system.math_unit.write_sqrt_param(val as _, MASK as _),
            MMIO_SQRT_PARAM2 => self.system.math_unit.write_sqrt_param((val as u64) << 32, (MASK as u64) << 32),
            MMIO_POSTFLG => {
                if MASK & 0xff != 0 {
                    self.write_postflg(val as u8)
                }
            }
            MMIO_POWCNT1 => self.system.video_unit.write_powcnt1(val, MASK),
            _ => warn!(
                "ARM9Memory: unmapped {}-bit write {:08x} = {:08x}",
                get_access_size(MASK),
                addr + get_access_offset(MASK),
                (val & MASK) >> (get_access_offset(MASK) * 8)
            ),
        }
    }
}

impl Arm9Memory {
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

    fn mmio_read<const MASK: u32>(&mut self, addr: u32) -> u32 {
        let mut val = 0;
        match mmio!(addr) {
            MMIO_DISPSTAT => {
                if MASK & 0xffff != 0 {
                    val |= self.system.video_unit.read_dispstat(Arch::ARMv5)
                }
                if MASK & 0xffff0000 != 0 {
                    val |= self.system.video_unit.read_vcount() << 16
                }
            }
            MMIO_DMAFILL_BASE..=MMIO_DMAFILL_END => return self.system.dma9.read_dmafill(addr),
            MMIO_KEYINPUT => {
                if MASK & 0xffff != 0 {
                    val |= self.system.input.read_keyinput() as u32
                }
                if MASK & 0xffff0000 != 0 {
                    error!("ARM9Memory: handle keycnt read")
                }
            }
            MMIO_IPCSYNC => return self.system.ipc.read_ipcsync(Arch::ARMv5),
            MMIO_IPCFIFOCNT => return self.system.ipc.read_ipcfifocnt(Arch::ARMv5) as u32,
            MMIO_IME => return self.system.arm9.get_irq().read_ime() as u32,
            MMIO_IE => return self.system.arm9.get_irq().read_ie(),
            MMIO_IRF => return self.system.arm9.get_irq().read_irf(),
            MMIO_DIVCNT => return self.system.math_unit.read_divcnt() as _,
            MMIO_DIV_NUMER => return self.system.math_unit.read_div_numer() as _,
            MMIO_DIV_NUMER2 => return (self.system.math_unit.read_div_numer() >> 32) as _,
            MMIO_DIV_DENOM => return self.system.math_unit.read_div_denom() as _,
            MMIO_DIV_DENOM2 => return (self.system.math_unit.read_div_denom() >> 32) as _,
            MMIO_DIV_RESULT => return self.system.math_unit.read_div_result() as _,
            MMIO_DIV_RESULT2 => return (self.system.math_unit.read_div_result() >> 32) as _,
            MMIO_DIV_REM_RESULT => return self.system.math_unit.read_divrem_result() as _,
            MMIO_DIV_REM_RESULT2 => return (self.system.math_unit.read_divrem_result() >> 32) as _,
            MMIO_SQRT_CNT => return self.system.math_unit.read_sqrtcnt() as _,
            MMIO_SQRT_RESULT => return self.system.math_unit.read_sqrt_result(),
            MMIO_SQRT_PARAM => return self.system.math_unit.read_sqrt_param() as u32,
            MMIO_SQRT_PARAM2 => return (self.system.math_unit.read_sqrt_param() >> 32) as _,
            MMIO_IPCFIFORECV => return self.system.ipc.read_ipcfiforecv(Arch::ARMv5),
            _ => warn!(
                "ARM9Memory: unmapped {}-bit read {:08x}",
                get_access_size(MASK),
                addr + get_access_offset(MASK),
            ),
        }
        val
    }
}
