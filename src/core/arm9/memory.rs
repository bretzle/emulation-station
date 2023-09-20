use std::mem::size_of;

use log::warn;

use crate::arm::coprocessor::Tcm;
use crate::arm::memory::{Memory, PageTable, RegionAttributes};
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

    pub itcm: Tcm,
    pub dtcm: Tcm,

    read_table: PageTable<14>,
    write_table: PageTable<14>,
}

impl Arm9Memory {
    pub fn new(system: &Shared<System>) -> Self {
        Self {
            system: system.clone(),
            postflg: 0,
            bios: vec![].into_boxed_slice(),
            dtcm_data: vec![0; 0x4000].into_boxed_slice(),
            itcm_data: vec![0; 0x8000].into_boxed_slice(),

            itcm: Tcm::default(),
            dtcm: Tcm::default(),

            read_table: PageTable::new(),
            write_table: PageTable::new(),
        }
    }

    pub fn reset(&mut self) {
        self.postflg = 0;
        self.dtcm_data.fill(0);
        self.itcm_data.fill(0);

        self.dtcm.data = self.dtcm_data.as_mut_ptr();
        self.itcm.data = self.itcm_data.as_mut_ptr();
        self.dtcm.mask = self.dtcm_data.len() as u32 - 1;
        self.itcm.mask = self.itcm_data.len() as u32 - 1;

        unsafe {
            let ptr = self.system.main_memory.as_mut_ptr();
            self.map(
                0x02000000,
                0x03000000,
                ptr,
                0x3fffff,
                RegionAttributes::ReadWrite,
            );
        }
        self.update_wram_mapping();
    }

    pub fn update_wram_mapping(&mut self) {
        match self.system.wramcnt {
            0x0 => warn!("update_wram_mapping"),
            0x1 => warn!("update_wram_mapping"),
            0x2 => warn!("update_wram_mapping"),
            0x3 => warn!("update_wram_mapping"),
            _ => unreachable!(),
        }
    }

    unsafe fn map(
        &mut self,
        base: u32,
        end: u32,
        ptr: *mut u8,
        mask: u32,
        attributes: RegionAttributes,
    ) {
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

    fn tcm_write<T>(&mut self, addr: u32, val: T) -> bool {
        let Self { itcm, dtcm, .. } = self;

        // TODO: if bus != system
        if itcm.enable_writes && addr >= itcm.base && addr < itcm.limit {
            // common::write<T>(itcm.data, value, (addr - itcm.config.base) & itcm.mask);
            // return;
            todo!()
        }

        // TODO: if bus = Data
        if dtcm.enable_writes && addr >= dtcm.base && addr < dtcm.limit {
            let ptr = dtcm.data;
            let val = val;
            let offset = (addr - dtcm.base) & dtcm.mask;
            unsafe {*ptr.add(offset as usize).cast() = val};
            // common::write<T>(dtcm.data, value, (addr - dtcm.config.base) & dtcm.mask);
            return true;
        }

        let ptr = self.write_table.get_pointer::<T>(addr);
        if !ptr.is_null() {
            unsafe { std::ptr::write(ptr as _, val) }
            return true;
        }

        false
    }

    fn tcm_read<T>(&mut self, addr: u32) -> Option<T> {
        let Self { itcm, dtcm, .. } = self;

        // TODO: if bus != System
        if itcm.enable_reads && addr >= itcm.base && addr < itcm.limit {
            // return common::read<T>(itcm.data, (addr - itcm.config.base) & itcm.mask);
            todo!()
        }

        // TODO: if bus = Data
        if dtcm.enable_reads && addr >= dtcm.base && addr < dtcm.limit {
            // return common::read<T>(dtcm.data, (addr - dtcm.config.base) & dtcm.mask);
            todo!()
        }

        let pointer = unsafe { self.read_table.get_pointer::<T>(addr) };
        if !pointer.is_null() {
            return Some(unsafe { std::ptr::read(pointer.cast()) });
        }

        None
    }

    fn mmio_write_byte(&mut self, addr: u32, val: u8) {
        let mirrored = val as u32 * 0x01010101;
        match addr & 0x3 {
            0x0 => self.mmio_write::<0x000000ff>(addr & !0x3, mirrored),
            0x1 => todo!(),
            0x2 => todo!(),
            0x3 => todo!(),
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
        const MMIO_POSTFLG: u32 = mmio!(0x04000300);
        const MMIO_POWCNT1: u32 = mmio!(0x04000304);

        match mmio!(addr) {
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
    fn read_byte(&mut self, addr: u32) -> u8 {
        let addr = addr & !(size_of::<u8>() as u32 - 1);
        todo!()
    }

    fn read_half(&mut self, addr: u32) -> u16 {
        let addr = addr & !(size_of::<u16>() as u32 - 1);
        todo!()
    }

    fn read_word(&mut self, addr: u32) -> u32 {
        let addr2 = addr & !(size_of::<u32>() as u32 - 1);
        if let Some(val) = self.tcm_read::<u32>(addr2) {
            return val;
        }

        match addr2 >> 24 {
            0x04 => todo!(),
            0x05 => todo!(),
            0x06 => self.system.video_unit.vram.read(addr2),
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
        let addr = addr & !(size_of::<u8>() as u32 - 1);

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
        let addr = addr & !(size_of::<u16>() as u32 - 1);
        if self.tcm_write(addr, val) {
            return;
        }
        match addr >> 24 {
            0x04 => self.mmio_write_half(addr, val),
            0x05 => todo!(),
            0x06 => todo!(),
            0x07 => todo!(),
            _ => warn!("ARM9Memory: handle 16-bit write {addr:08x} = {val:04x}"),
        }
    }

    fn write_word(&mut self, addr: u32, val: u32) {
        let addr = addr & !(size_of::<u32>() as u32 - 1);
        if self.tcm_write(addr, val) {
            return;
        }
        match addr >> 24 {
            0x00 | 0x01 => {}
            0x04 => self.mmio_write_word(addr, val),
            0x05 => todo!(),
            0x06 => todo!(),
            0x07 => todo!(),
            0x08 | 0x09 => todo!(),
            _ => warn!("ARM9Memory: handle 32-bit write {addr:08x} = {val:08x}"),
        }
    }
}
