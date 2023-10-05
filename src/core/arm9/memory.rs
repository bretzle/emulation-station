use std::any::Any;

use log::{error, warn};

use crate::arm::coprocessor::Tcm;
use crate::arm::cpu::Arch;
use crate::arm::memory::{Memory, MmioMemory};
use crate::core::System;
use crate::core::video::vram::VramBank;
use crate::util::*;

macro_rules! mmio {
    ($x:tt) => {
        $x >> 2
    };
}

macro_rules! handle {
    ($mask:ident => {
        $( $filter:literal: $call:expr ),+ $(,)?
    }) => {{
        { $( if $mask & $filter != 0 { $call } )+ }
    }};
}

const MMIO_DISPCNT: u32 = mmio!(0x04000000);
const MMIO_DISPSTAT: u32 = mmio!(0x04000004);
const MMIO_PPUA_BGCNT0: u32 = mmio!(0x04000008);
const MMIO_PPUA_BGCNT1: u32 = mmio!(0x0400000c);
const MMIO_PPUA_BGHOFS0: u32 = mmio!(0x04000010);
const MMIO_PPUA_BGHOFS1: u32 = mmio!(0x04000014);
const MMIO_PPUA_BGHOFS2: u32 = mmio!(0x04000018);
const MMIO_PPUA_BGHOFS3: u32 = mmio!(0x0400001c);
const MMIO_PPUA_BGPA0: u32 = mmio!(0x04000020);
const MMIO_PPUA_BGPC0: u32 = mmio!(0x04000024);
const MMIO_PPUA_BGX0: u32 = mmio!(0x04000028);
const MMIO_PPUA_BGY0: u32 = mmio!(0x0400002c);
const MMIO_PPUA_BGPA1: u32 = mmio!(0x04000030);
const MMIO_PPUA_BGPC1: u32 = mmio!(0x04000034);
const MMIO_PPUA_BGX1: u32 = mmio!(0x04000038);
const MMIO_PPUA_BGY1: u32 = mmio!(0x0400003c);
const MMIO_PPUA_WINH: u32 = mmio!(0x04000040);
const MMIO_PPUA_WINV: u32 = mmio!(0x04000044);
const MMIO_PPUA_WININ: u32 = mmio!(0x04000048);
const MMIO_PPUA_MOSAIC: u32 = mmio!(0x0400004c);
const MMIO_PPUA_BLDCNT: u32 = mmio!(0x04000050);
const MMIO_PPUA_BLDY: u32 = mmio!(0x04000054);
const MMIO_PPUA_RESERVED0: u32 = mmio!(0x04000058);
const MMIO_PPUA_RESERVED1: u32 = mmio!(0x0400005c);
const MMIO_GPU_DISP3DCNT: u32 = mmio!(0x04000060);
const MMIO_DISPCAPCNT: u32 = mmio!(0x04000064);
const MMIO_PPUA_MASTERBRIGHT: u32 = mmio!(0x0400006c);
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
const MMIO_TIMER0: u32 = mmio!(0x04000100);
const MMIO_TIMER1: u32 = mmio!(0x04000104);
const MMIO_TIMER2: u32 = mmio!(0x04000108);
const MMIO_TIMER3: u32 = mmio!(0x0400010c);
const MMIO_KEYINPUT: u32 = mmio!(0x04000130);
const MMIO_IPCSYNC: u32 = mmio!(0x04000180);
const MMIO_IPCFIFOCNT: u32 = mmio!(0x04000184);
const MMIO_IPCFIFOSEND: u32 = mmio!(0x04000188);
const MMIO_EXMEMCNT: u32 = mmio!(0x04000204);
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
const MMIO_PPUB_DISPCNT: u32 = mmio!(0x04001000);
const MMIO_PPUB_RESERVED0: u32 = mmio!(0x04001004);
const MMIO_PPUB_BGCNT0: u32 = mmio!(0x04001008);
const MMIO_PPUB_BGCNT1: u32 = mmio!(0x0400100c);
const MMIO_PPUB_BGHOFS0: u32 = mmio!(0x04001010);
const MMIO_PPUB_BGHOFS1: u32 = mmio!(0x04001014);
const MMIO_PPUB_BGHOFS2: u32 = mmio!(0x04001018);
const MMIO_PPUB_BGHOFS3: u32 = mmio!(0x0400101c);
const MMIO_PPUB_BGPA0: u32 = mmio!(0x04001020);
const MMIO_PPUB_BGPC0: u32 = mmio!(0x04001024);
const MMIO_PPUB_BGX0: u32 = mmio!(0x04001028);
const MMIO_PPUB_BGY0: u32 = mmio!(0x0400102c);
const MMIO_PPUB_BGPA1: u32 = mmio!(0x04001030);
const MMIO_PPUB_BGPC1: u32 = mmio!(0x04001034);
const MMIO_PPUB_BGX1: u32 = mmio!(0x04001038);
const MMIO_PPUB_BGY1: u32 = mmio!(0x0400103c);
const MMIO_PPUB_WINH: u32 = mmio!(0x04001040);
const MMIO_PPUB_WINV: u32 = mmio!(0x04001044);
const MMIO_PPUB_WININ: u32 = mmio!(0x04001048);
const MMIO_PPUB_MOSAIC: u32 = mmio!(0x0400104c);
const MMIO_PPUB_BLDCNT: u32 = mmio!(0x04001050);
const MMIO_PPUB_BLDY: u32 = mmio!(0x04001054);
const MMIO_PPUB_RESERVED_START: u32 = mmio!(0x04001058);
const MMIO_PPUB_RESERVED_END: u32 = mmio!(0x04001068);
const MMIO_PPUB_MASTERBRIGHT: u32 = mmio!(0x0400106c);
const MMIO_IPCFIFORECV: u32 = mmio!(0x04100000);

pub struct Arm9Memory {
    system: Shared<System>,
    postflg: u8,
    bios: Box<[u8]>,
    dtcm_data: Box<[u8]>,
    itcm_data: Box<[u8]>,

    pub itcm: Shared<Tcm>,
    pub dtcm: Shared<Tcm>,

    pages: PageTable<14>,
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

            pages: PageTable::new(),
        }
    }

    pub fn update_wram_mapping(&mut self) {
        match self.system.wramcnt {
            0x0 => self.pages.map(
                0x03000000,
                0x04000000,
                self.system.shared_wram.as_ptr() as _,
                0x7fff,
                RegionAttributes::ReadWrite,
            ),
            0x1 => self.pages.map(
                0x03000000,
                0x04000000,
                unsafe { self.system.shared_wram.as_ptr().add(0x4000) as _ },
                0x3fff,
                RegionAttributes::ReadWrite,
            ),
            0x2 => self.pages.map(
                0x03000000,
                0x04000000,
                self.system.shared_wram.as_ptr() as _,
                0x3fff,
                RegionAttributes::ReadWrite,
            ),
            0x3 => self.pages.unmap(0x03000000, 0x04000000, RegionAttributes::ReadWrite),
            _ => unreachable!(),
        }
    }

    fn tcm_write<T>(&mut self, addr: u32, val: T) -> bool {
        let Self { itcm, dtcm, .. } = self;

        // TODO: if bus != system
        if itcm.enable_writes && addr >= itcm.base && addr < itcm.limit {
            let offset = (addr - itcm.base) & itcm.mask;
            unsafe { *itcm.data.add(offset as usize).cast() = val };
            return true;
        }

        // TODO: if bus = Data
        if dtcm.enable_writes && addr >= dtcm.base && addr < dtcm.limit {
            let offset = (addr - dtcm.base) & dtcm.mask;
            unsafe { *dtcm.data.add(offset as usize).cast() = val };
            return true;
        }

        let ptr = self.pages.write_pointer::<T>(addr);
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
            return Some(unsafe {
                let offset = (addr - itcm.base) & itcm.mask;
                *itcm.data.add(offset as usize).cast::<T>()
            });
        }

        // TODO: if bus = Data
        if dtcm.enable_reads && addr >= dtcm.base && addr < dtcm.limit {
            return Some(unsafe {
                let offset = (addr - dtcm.base) & dtcm.mask;
                *dtcm.data.add(offset as usize).cast::<T>()
            });
        }

        let ptr = self.pages.read_pointer::<T>(addr);
        if !ptr.is_null() {
            return Some(unsafe { std::ptr::read(ptr) });
        }

        None
    }

    fn write_postflg(&mut self, val: u8) {
        self.postflg = (self.postflg & !0x2) | (val & 0x3)
    }
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
            self.pages.map(0xffff0000, 0xffff8000, ptr, 0x7fff, RegionAttributes::Read);
            let ptr = self.system.main_memory.as_mut_ptr();
            self.pages.map(0x02000000, 0x03000000, ptr, 0x3fffff, RegionAttributes::ReadWrite);
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
            0x08 | 0x09 => {
                if bit::<7>(self.system.exmemcnt as _) {
                    0
                } else {
                    0xffff
                }
            }
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

impl MmioMemory for Arm9Memory {
    fn mmio_read<const MASK: u32>(&mut self, addr: u32) -> u32 {
        let mut val = 0;

        match mmio!(addr) {
            MMIO_DISPCNT => return self.system.video_unit.ppu_a.read_dispcnt(),
            MMIO_DISPSTAT => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.read_dispstat(Arch::ARMv5),
                0xffff0000: val |= self.system.video_unit.read_vcount() << 16
            }},
            MMIO_PPUA_BGCNT0 => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.ppu_a.read_bgcnt(0) as u32,
                0xffff0000: val |= (self.system.video_unit.ppu_a.read_bgcnt(1) as u32) << 16
            }},
            MMIO_PPUA_BGCNT1 => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.ppu_a.read_bgcnt(2) as u32,
                0xffff0000: val |= (self.system.video_unit.ppu_a.read_bgcnt(3) as u32) << 16
            }},
            MMIO_PPUA_WININ => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.ppu_a.read_winin() as u32,
                0xffff0000: val |= (self.system.video_unit.ppu_a.read_winout() as u32) << 16
            }},
            MMIO_DMA_SOURCE0 => return self.system.dma9.read_source(0),
            MMIO_DMA_LENGTH0 => handle! { MASK => {
                0x0000ffff: val |= self.system.dma9.read_length(0),
                0xffff0000: val |= (self.system.dma9.read_control(0) as u32) << 16
            }},
            MMIO_DMA_LENGTH1 => handle! { MASK => {
                0x0000ffff: val |= self.system.dma9.read_length(1),
                0xffff0000: val |= (self.system.dma9.read_control(1) as u32) << 16
            }},
            MMIO_DMA_LENGTH2 => handle! { MASK => {
                0x0000ffff: val |= self.system.dma9.read_length(2),
                0xffff0000: val |= (self.system.dma9.read_control(2) as u32) << 16
            }},
            MMIO_DMA_LENGTH3 => handle! { MASK => {
                0x0000ffff: val |= self.system.dma9.read_length(3),
                0xffff0000: val |= (self.system.dma9.read_control(3) as u32) << 16
            }},
            MMIO_DMAFILL_BASE..=MMIO_DMAFILL_END => return self.system.dma9.read_dmafill(addr),
            MMIO_TIMER0 => handle! { MASK => {
                0x0000ffff: val |= self.system.timer9.read_length(0) as u32,
                0xffff0000: val |= (self.system.timer9.read_control(0) as u32) << 16
            }},
            MMIO_TIMER1 => handle! { MASK => {
                0x0000ffff: val |= self.system.timer9.read_length(1) as u32,
                0xffff0000: val |= (self.system.timer9.read_control(1) as u32) << 16
            }},
            MMIO_TIMER2 => handle! { MASK => {
                0x0000ffff: val |= self.system.timer9.read_length(2) as u32,
                0xffff0000: val |= (self.system.timer9.read_control(2) as u32) << 16
            }},
            MMIO_TIMER3 => handle! { MASK => {
                0x0000ffff: val |= self.system.timer9.read_length(3) as u32,
                0xffff0000: val |= (self.system.timer9.read_control(3) as u32) << 16
            }},
            MMIO_KEYINPUT => handle! { MASK => {
                0x0000ffff: val |= self.system.input.read_keyinput() as u32,
                0xffff0000: error!("ARM9Memory: handle keycnt read")
            }},
            MMIO_IPCSYNC => return self.system.ipc.read_ipcsync(Arch::ARMv5),
            MMIO_IPCFIFOCNT => return self.system.ipc.read_ipcfifocnt(Arch::ARMv5) as u32,
            MMIO_EXMEMCNT => return self.system.read_exmemcnt() as u32,
            MMIO_IME => return self.system.arm9.get_irq().read_ime() as u32,
            MMIO_IE => return self.system.arm9.get_irq().read_ie(),
            MMIO_IRF => return self.system.arm9.get_irq().read_irf(),
            MMIO_VRAMCNT => handle! { MASK => {
                0x000000ff: val |= self.system.video_unit.vram.read_vramcnt(VramBank::A) as u32,
                0x0000ff00: val |= (self.system.video_unit.vram.read_vramcnt(VramBank::B) as u32) << 8,
                0x00ff0000: val |= (self.system.video_unit.vram.read_vramcnt(VramBank::C) as u32) << 16,
                0xff000000: val |= (self.system.video_unit.vram.read_vramcnt(VramBank::D) as u32) << 24
            }},
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
            MMIO_POSTFLG => handle! { MASK => {
                0xff: val |= self.postflg as u32
            }},
            MMIO_POWCNT1 => return self.system.video_unit.read_powcnt1(),
            MMIO_PPUB_DISPCNT => return self.system.video_unit.ppu_b.read_dispcnt(),
            MMIO_PPUB_BGCNT0 => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.ppu_b.read_bgcnt(0) as u32,
                0xffff0000: val |= (self.system.video_unit.ppu_b.read_bgcnt(1) as u32) << 16
            }},
            MMIO_PPUB_BGCNT1 => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.ppu_b.read_bgcnt(2) as u32,
                0xffff0000: val |= (self.system.video_unit.ppu_b.read_bgcnt(3) as u32) << 16
            }},
            MMIO_PPUB_WININ => handle! { MASK => {
                0x0000ffff: val |= self.system.video_unit.ppu_b.read_winin() as u32,
                0xffff0000: val |= (self.system.video_unit.ppu_b.read_winout() as u32) << 16
            }},
            MMIO_IPCFIFORECV => return self.system.ipc.read_ipcfiforecv(Arch::ARMv5),
            _ => warn!(
                "ARM9Memory: unmapped {}-bit  read {:08x}",
                get_access_size(MASK),
                addr + get_access_offset(MASK),
            ),
        }
        val
    }

    fn mmio_write<const MASK: u32>(&mut self, addr: u32, val: u32) {
        match mmio!(addr) {
            MMIO_DISPCNT => self.system.video_unit.ppu_a.write_dispcnt(val, MASK),
            MMIO_DISPSTAT => handle! { MASK => {
                0x0000ffff: self.system.video_unit.write_dispstat(Arch::ARMv5, val, MASK),
                0xffff0000: self.system.video_unit.write_vcount((val >> 16) as u16, (MASK >> 16) as u16)
            }},
            MMIO_PPUA_BGCNT0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bgcnt(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgcnt(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGCNT1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bgcnt(2, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgcnt(3, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGHOFS0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bghofs(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgvofs(0, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGHOFS1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bghofs(1, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgvofs(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGHOFS2 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bghofs(2, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgvofs(2, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGHOFS3 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bghofs(3, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgvofs(3, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGPA0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bgpa(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgpb(0, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGPC0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bgpc(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgpd(0, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGX0 => self.system.video_unit.ppu_a.write_bgx(0, val, MASK),
            MMIO_PPUA_BGY0 => self.system.video_unit.ppu_a.write_bgy(0, val, MASK),
            MMIO_PPUA_BGPA1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bgpa(1, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgpb(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGPC1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bgpc(1, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bgpd(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BGX1 => self.system.video_unit.ppu_a.write_bgx(1, val, MASK),
            MMIO_PPUA_BGY1 => self.system.video_unit.ppu_a.write_bgy(1, val, MASK),
            MMIO_PPUA_WINH => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_winh(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_winh(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_WINV => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_winv(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_winv(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_WININ => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_winin(val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_winout((val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_MOSAIC => self.system.video_unit.ppu_a.write_mosaic(val as _, MASK as _),
            MMIO_PPUA_BLDCNT => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_a.write_bldcnt(val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_a.write_bldalpha((val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUA_BLDY => self.system.video_unit.ppu_a.write_bldy(val as _, MASK as _),
            MMIO_PPUA_RESERVED0 | MMIO_PPUA_RESERVED1 => {}
            MMIO_GPU_DISP3DCNT => { /* todo: gpu */ }
            MMIO_DISPCAPCNT => self.system.video_unit.write_dispcapcnt(val, MASK),
            MMIO_PPUA_MASTERBRIGHT => self.system.video_unit.ppu_a.write_master_bright(val, MASK),
            MMIO_DMA_SOURCE0 => self.system.dma9.write_source(0, val, MASK),
            MMIO_DMA_DESTINATION0 => self.system.dma9.write_destination(0, val, MASK),
            MMIO_DMA_LENGTH0 => handle! { MASK => {
                0x0000ffff: self.system.dma9.write_length(0, val, MASK),
                0xffff0000: self.system.dma9.write_control(0, val >> 16, MASK >> 16)
            }},
            MMIO_DMA_SOURCE1 => self.system.dma9.write_source(1, val, MASK),
            MMIO_DMA_DESTINATION1 => self.system.dma9.write_destination(1, val, MASK),
            MMIO_DMA_LENGTH1 => handle! { MASK => {
                0x0000ffff: self.system.dma9.write_length(1, val, MASK),
                0xffff0000: self.system.dma9.write_control(1, val >> 16, MASK >> 16)
            }},
            MMIO_DMA_SOURCE2 => self.system.dma9.write_source(2, val, MASK),
            MMIO_DMA_DESTINATION2 => self.system.dma9.write_destination(2, val, MASK),
            MMIO_DMA_LENGTH2 => handle! { MASK => {
                0x0000ffff: self.system.dma9.write_length(2, val, MASK),
                0xffff0000: self.system.dma9.write_control(2, val >> 16, MASK >> 16)
            }},
            MMIO_DMA_SOURCE3 => self.system.dma9.write_source(3, val, MASK),
            MMIO_DMA_DESTINATION3 => self.system.dma9.write_destination(3, val, MASK),
            MMIO_DMA_LENGTH3 => handle! { MASK => {
                0x0000ffff: self.system.dma9.write_length(3, val, MASK),
                0xffff0000: self.system.dma9.write_control(3, val >> 16, MASK >> 16)
            }},
            MMIO_DMAFILL_BASE..=MMIO_DMAFILL_END => self.system.dma9.write_dmafill(addr, val),
            MMIO_TIMER0 => handle! { MASK => {
                0x0000ffff: self.system.timer9.write_length(0, val, MASK),
                0xffff0000: self.system.timer9.write_control(0, (val >> 16) as u16, MASK >> 16)
            }},
            MMIO_TIMER1 => handle! { MASK => {
                0x0000ffff: self.system.timer9.write_length(1, val, MASK),
                0xffff0000: self.system.timer9.write_control(1, (val >> 16) as u16, MASK >> 16)
            }},
            MMIO_TIMER2 => handle! { MASK => {
                0x0000ffff: self.system.timer9.write_length(2, val, MASK),
                0xffff0000: self.system.timer9.write_control(2, (val >> 16) as u16, MASK >> 16)
            }},
            MMIO_TIMER3 => handle! { MASK => {
                0x0000ffff: self.system.timer9.write_length(3, val, MASK),
                0xffff0000: self.system.timer9.write_control(3, (val >> 16) as u16, MASK >> 16)
            }},
            MMIO_IPCSYNC => handle! { MASK => {
                0xffff: self.system.ipc.write_ipcsync(Arch::ARMv5, val, MASK)
            }},
            MMIO_IPCFIFOCNT => handle! { MASK => {
                0xffff: self.system.ipc.write_ipcfifocnt(Arch::ARMv5, val as _, MASK as _)
            }},
            MMIO_IPCFIFOSEND => self.system.ipc.write_ipcfifosend(Arch::ARMv5, val),
            MMIO_EXMEMCNT => handle! { MASK => {
                0xffff: self.system.write_exmemcnt(val as _, MASK as _)
            }},
            MMIO_IME => self.system.arm9.get_irq().write_ime(val, MASK),
            MMIO_IE => self.system.arm9.get_irq().write_ie(val, MASK),
            MMIO_IRF => self.system.arm9.get_irq().write_irf(val, MASK),
            MMIO_VRAMCNT => handle! { MASK => {
                0x000000ff: self.system.video_unit.vram.write_vramcnt(VramBank::A, val as u8),
                0x0000ff00: self.system.video_unit.vram.write_vramcnt(VramBank::B, (val >> 8) as u8),
                0x00ff0000: self.system.video_unit.vram.write_vramcnt(VramBank::C, (val >> 16) as u8),
                0xff000000: self.system.video_unit.vram.write_vramcnt(VramBank::D, (val >> 24) as u8)
            }},
            MMIO_VRAMCNT2 => handle! { MASK => {
                0x000000ff: self.system.video_unit.vram.write_vramcnt(VramBank::E, val as u8),
                0x0000ff00: self.system.video_unit.vram.write_vramcnt(VramBank::F, (val >> 8) as u8),
                0x00ff0000: self.system.video_unit.vram.write_vramcnt(VramBank::G, (val >> 16) as u8),
                0xff000000: self.system.write_wramcnt((val >> 24) as u8)
            }},
            MMIO_VRAMCNT3 => handle! { MASK => {
                0x00ff: self.system.video_unit.vram.write_vramcnt(VramBank::H, val as u8),
                0xff00: self.system.video_unit.vram.write_vramcnt(VramBank::I, (val >> 8) as u8)
            }},
            MMIO_DIVCNT => self.system.math_unit.write_divcnt(val as _, MASK as _),
            MMIO_DIV_NUMER => self.system.math_unit.write_div_numer(val as _, MASK as _),
            MMIO_DIV_NUMER2 => self.system.math_unit.write_div_numer((val as u64) << 32, (MASK as u64) << 32),
            MMIO_DIV_DENOM => self.system.math_unit.write_div_denom(val as _, MASK as _),
            MMIO_DIV_DENOM2 => self.system.math_unit.write_div_denom((val as u64) << 32, (MASK as u64) << 32),
            MMIO_SQRT_CNT => self.system.math_unit.write_sqrtcnt(val as _, MASK as _),
            MMIO_SQRT_PARAM => self.system.math_unit.write_sqrt_param(val as _, MASK as _),
            MMIO_SQRT_PARAM2 => self.system.math_unit.write_sqrt_param((val as u64) << 32, (MASK as u64) << 32),
            MMIO_POSTFLG => handle! { MASK => {
                0xff: self.write_postflg(val as u8)
            }},
            MMIO_POWCNT1 => self.system.video_unit.write_powcnt1(val, MASK),
            MMIO_PPUB_DISPCNT => self.system.video_unit.ppu_b.write_dispcnt(val, MASK),
            MMIO_PPUB_RESERVED0 => {}
            MMIO_PPUB_BGCNT0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bgcnt(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgcnt(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGCNT1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bgcnt(2, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgcnt(3, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGHOFS0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bghofs(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgvofs(0, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGHOFS1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bghofs(1, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgvofs(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGHOFS2 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bghofs(2, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgvofs(2, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGHOFS3 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bghofs(3, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgvofs(3, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGPA0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bgpa(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgpb(0, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGPC0 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bgpc(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgpd(0, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGX0 => self.system.video_unit.ppu_b.write_bgx(0, val, MASK),
            MMIO_PPUB_BGY0 => self.system.video_unit.ppu_b.write_bgy(0, val, MASK),
            MMIO_PPUB_BGPA1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bgpa(1, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgpb(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGPC1 => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bgpc(1, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bgpd(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BGX1 => self.system.video_unit.ppu_b.write_bgx(1, val, MASK),
            MMIO_PPUB_BGY1 => self.system.video_unit.ppu_b.write_bgy(1, val, MASK),
            MMIO_PPUB_WINH => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_winh(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_winh(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_WINV => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_winv(0, val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_winv(1, (val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_WININ => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_winin(val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_winout((val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_MOSAIC => self.system.video_unit.ppu_b.write_mosaic(val as _, MASK as _),
            MMIO_PPUB_BLDCNT => handle! { MASK => {
                0x0000ffff: self.system.video_unit.ppu_b.write_bldcnt(val as _, MASK as _),
                0xffff0000: self.system.video_unit.ppu_b.write_bldalpha((val >> 16) as _, (MASK >> 16) as _)
            }},
            MMIO_PPUB_BLDY => self.system.video_unit.ppu_b.write_bldy(val as _, MASK as _),
            MMIO_PPUB_RESERVED_START..=MMIO_PPUB_RESERVED_END => {}
            MMIO_PPUB_MASTERBRIGHT => self.system.video_unit.ppu_b.write_master_bright(val, MASK),
            _ => warn!(
                "ARM9Memory: unmapped {}-bit write {:08x} = {:08x}",
                get_access_size(MASK),
                addr + get_access_offset(MASK),
                (val & MASK) >> (get_access_offset(MASK) * 8)
            ),
        }
    }
}
