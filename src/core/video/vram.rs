use crate::bitfield;
use crate::util::Shared;

use std::fmt::Debug;

use std::ops::BitOrAssign;

pub enum VramBank {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
}

bitfield! {
    #[derive(Clone, Copy, Default)]
    struct VramCnt(u8) {
        mst: u8 => 0 | 2,
        offset: u8 => 3 | 4,
        // 5 | 6
        enable: bool => 7
    }
}

pub struct Vram {
    pub lcdc: Shared<VramRegion>,
    pub bga: Shared<VramRegion>,
    pub bgb: Shared<VramRegion>,
    pub obja: Shared<VramRegion>,
    pub objb: Shared<VramRegion>,
    pub arm7_vram: VramRegion,
    pub texture_data: VramRegion,
    pub texture_palette: VramRegion,
    pub bga_extended_palette: Shared<VramRegion>,
    pub bgb_extended_palette: Shared<VramRegion>,
    pub obja_extended_palette: Shared<VramRegion>,
    pub objb_extended_palette: Shared<VramRegion>,

    vramstat: u8,

    vramcnt: [VramCnt; 9],

    bank_a: Box<[u8; 0x20000]>,
    bank_b: Box<[u8; 0x20000]>,
    bank_c: Box<[u8; 0x20000]>,
    bank_d: Box<[u8; 0x20000]>,
    bank_e: Box<[u8; 0x10000]>,
    bank_f: Box<[u8; 0x4000]>,
    bank_g: Box<[u8; 0x4000]>,
    bank_h: Box<[u8; 0x8000]>,
    bank_i: Box<[u8; 0x4000]>,
}

impl Vram {
    pub fn new() -> Self {
        Self {
            lcdc: Default::default(),
            bga: Default::default(),
            obja: Default::default(),
            bgb: Default::default(),
            objb: Default::default(),
            arm7_vram: Default::default(),
            texture_data: Default::default(),
            texture_palette: Default::default(),
            bga_extended_palette: Default::default(),
            bgb_extended_palette: Default::default(),
            obja_extended_palette: Default::default(),
            objb_extended_palette: Default::default(),
            vramstat: 0,
            vramcnt: [VramCnt(0); 9],
            bank_a: Box::new([0; 0x20000]),
            bank_b: Box::new([0; 0x20000]),
            bank_c: Box::new([0; 0x20000]),
            bank_d: Box::new([0; 0x20000]),
            bank_e: Box::new([0; 0x10000]),
            bank_f: Box::new([0; 0x4000]),
            bank_g: Box::new([0; 0x4000]),
            bank_h: Box::new([0; 0x8000]),
            bank_i: Box::new([0; 0x4000]),
        }
    }

    pub fn reset(&mut self) {
        self.lcdc.allocate(0xa4000);
        self.bga.allocate(0x80000);
        self.obja.allocate(0x40000);
        self.bgb.allocate(0x20000);
        self.objb.allocate(0x20000);
        self.arm7_vram.allocate(0x40000);
        self.texture_data.allocate(0x80000);
        self.texture_palette.allocate(0x20000);
        self.bga_extended_palette.allocate(0x8000);
        self.bgb_extended_palette.allocate(0x8000);
        self.obja_extended_palette.allocate(0x2000);
        self.objb_extended_palette.allocate(0x2000);

        self.reset_regions();
    }

    fn reset_regions(&mut self) {
        self.lcdc.reset();
        self.bga.reset();
        self.obja.reset();
        self.bgb.reset();
        self.objb.reset();
        self.arm7_vram.reset();
        self.texture_data.reset();
        self.texture_palette.reset();
        self.bga_extended_palette.reset();
        self.bgb_extended_palette.reset();
    }

    pub fn read(&mut self, addr: u32) -> u32 {
        let region = (addr >> 20) & 0xf;
        match region {
            0x0 | 0x1 => self.bga.read(addr),
            0x2 | 0x3 => self.bgb.read(addr),
            0x4 | 0x5 => self.obja.read(addr),
            0x6 | 0x7 => self.objb.read(addr),
            _ => self.lcdc.read(addr),
        }
    }

    pub fn write<T: Copy + Debug + Into<u32>>(&mut self, addr: u32, val: T) {
        let region = (addr >> 20) & 0xf;
        match region {
            0x0 | 0x1 => self.bga.write(addr, val),
            0x2 | 0x3 => self.bgb.write(addr, val),
            0x4 | 0x5 => self.obja.write(addr, val),
            0x6 | 0x7 => self.objb.write(addr, val),
            _ => self.lcdc.write(addr, val),
        }
    }

    pub const fn read_vramstat(&self) -> u8 {
        self.vramstat
    }

    pub fn read_vramcnt(&self, bank: VramBank) -> u8 {
        self.vramcnt[bank as usize].0
    }

    pub fn write_vramcnt(&mut self, bank: VramBank, mut val: u8) {
        let masks = [0x9b, 0x9b, 0x9f, 0x9f, 0x87, 0x9f, 0x9f, 0x83, 0x83];
        let index = bank as usize;
        val &= masks[index];

        if self.vramcnt[index].0 == val {
            return;
        }

        self.vramcnt[index].0 = val;
        self.reset_regions();

        if self.vramcnt[0].enable() {
            let ptr = self.bank_a.as_mut_ptr();
            let offset = self.vramcnt[0].offset() as usize;
            match self.vramcnt[0].mst() {
                0 => self.lcdc.map(ptr, 0, 0x20000),
                1 => self.bga.map(ptr, offset * 0x20000, 0x20000),
                2 => self.obja.map(ptr, (offset & 1) * 0x20000, 0x20000),
                3 => self.texture_data.map(ptr, offset * 0x20000, 0x20000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[1].enable() {
            let ptr = self.bank_b.as_mut_ptr();
            let offset = self.vramcnt[1].offset() as usize;
            match self.vramcnt[1].mst() {
                0 => self.lcdc.map(ptr, 0x20000, 0x20000),
                1 => self.bga.map(ptr, offset * 0x20000, 0x20000),
                2 => self.obja.map(ptr, (offset & 1) * 0x20000, 0x20000),
                3 => self.texture_data.map(ptr, offset * 0x20000, 0x20000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[2].enable() {
            let ptr = self.bank_c.as_mut_ptr();
            let offset = self.vramcnt[2].offset() as usize;
            match self.vramcnt[2].mst() {
                0 => self.lcdc.map(ptr, 0x40000, 0x20000),
                1 => self.bga.map(ptr, offset * 0x20000, 0x20000),
                2 => self.arm7_vram.map(ptr, (offset & 1) * 0x20000, 0x20000),
                3 => self.texture_data.map(ptr, offset * 0x20000, 0x20000),
                4 => self.bgb.map(ptr, 0, 0x20000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[2].enable() && self.vramcnt[2].mst() == 2 {
            self.vramstat |= 1;
        } else {
            self.vramstat &= !1;
        }

        if self.vramcnt[3].enable() {
            let ptr = self.bank_d.as_mut_ptr();
            let offset = self.vramcnt[3].offset() as usize;
            match self.vramcnt[3].mst() {
                0 => self.lcdc.map(ptr, 0x60000, 0x20000),
                1 => self.bga.map(ptr, offset * 0x20000, 0x20000),
                2 => self.arm7_vram.map(ptr, (offset & 1) * 0x20000, 0x20000),
                3 => self.texture_data.map(ptr, offset * 0x20000, 0x20000),
                4 => self.objb.map(ptr, 0, 0x20000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[3].enable() && self.vramcnt[3].mst() == 2 {
            self.vramstat |= 1 << 1
        } else {
            self.vramstat &= !(1 << 1);
        }

        if self.vramcnt[4].enable() {
            let ptr = self.bank_e.as_mut_ptr();
            match self.vramcnt[4].mst() {
                0 => self.lcdc.map(ptr, 0x80000, 0x10000),
                1 => self.bga.map(ptr, 0, 0x10000),
                2 => self.obja.map(ptr, 0, 0x10000),
                3 => self.texture_palette.map(ptr, 0, 0x10000),
                4 => self.bga_extended_palette.map(ptr, 0, 0x8000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[5].enable() {
            let ptr = self.bank_f.as_mut_ptr();
            let offset = self.vramcnt[5].offset() as usize;
            match self.vramcnt[5].mst() {
                0 => self.lcdc.map(ptr, 0x90000, 0x4000),
                1 => self.bga.map(ptr, (offset & 1) * 0x4000 + (offset & 2) * 0x10000, 0x4000),
                2 => self.obja.map(ptr, (offset & 1) * 0x4000 + (offset & 2) * 0x10000, 0x4000),
                3 => self.texture_palette.map(ptr, ((offset & 1) + (offset & 2) * 4) * 0x4000, 0x4000),
                4 => self.bga_extended_palette.map(ptr, (offset & 1) * 0x4000, 0x4000),
                5 => self.obja_extended_palette.map(ptr, 0, 0x2000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[6].enable() {
            let ptr = self.bank_g.as_mut_ptr();
            let offset = self.vramcnt[6].offset() as usize;
            match self.vramcnt[6].mst() {
                0 => self.lcdc.map(ptr, 0x94000, 0x4000),
                1 => self.bga.map(ptr, (offset & 1) * 0x4000 + (offset & 2) * 0x10000, 0x4000),
                2 => self.obja.map(ptr, (offset & 1) * 0x4000 + (offset & 2) * 0x10000, 0x4000),
                3 => self.texture_palette.map(ptr, ((offset & 1) + (offset & 2) * 4) * 0x4000, 0x4000),
                4 => self.bga_extended_palette.map(ptr, (offset & 1) * 0x4000, 0x4000),
                5 => self.obja_extended_palette.map(ptr, 0, 0x2000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[7].enable() {
            let ptr = self.bank_h.as_mut_ptr();
            match self.vramcnt[7].mst() {
                0 => self.lcdc.map(ptr, 0x98000, 0x8000),
                1 => self.bgb.map(ptr, 0, 0x8000),
                2 => self.bgb_extended_palette.map(ptr, 0, 0x8000),
                _ => unreachable!(),
            }
        }

        if self.vramcnt[8].enable() {
            let ptr = self.bank_i.as_mut_ptr();
            match self.vramcnt[8].mst() {
                0 => self.lcdc.map(ptr, 0xa0000, 0x4000),
                1 => self.bgb.map(ptr, 0x8000, 0x4000),
                2 => self.objb.map(ptr, 0, 0x4000),
                3 => self.objb_extended_palette.map(ptr, 0, 0x2000),
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Default)]
pub struct VramPage {
    banks: Vec<*mut u8>,
}

impl VramPage {
    const PAGE_SIZE: u32 = 0x1000;
    const PAGE_MASK: u32 = Self::PAGE_SIZE - 1;

    pub fn reset(&mut self) {
        self.banks.clear()
    }

    pub fn add_bank(&mut self, ptr: *mut u8) {
        self.banks.push(ptr);
    }

    pub fn read<T: Default + BitOrAssign + Copy>(&mut self, addr: u32) -> T {
        unsafe {
            let mut data = T::default();
            for bank in &self.banks {
                let offset = (addr & Self::PAGE_MASK) as usize;
                let ptr = bank.add(offset).cast::<T>();
                data |= *ptr;
            }
            data
        }
    }

    pub fn write<T: Copy>(&mut self, addr: u32, val: T) {
        unsafe {
            for bank in self.banks.iter().copied() {
                let offset = (addr & Self::PAGE_MASK) as usize;
                let ptr = bank.add(offset).cast::<T>();
                *ptr = val
            }
        }
    }
}

#[derive(Default)]
pub struct VramRegion {
    pages: Vec<VramPage>,
}

impl VramRegion {
    const PAGE_SIZE: usize = 0x1000;

    pub fn reset(&mut self) {
        for page in &mut self.pages {
            page.reset();
        }
    }

    pub fn read<T: Default + BitOrAssign + Copy>(&mut self, addr: u32) -> T {
        self.get_page(addr).read(addr)
    }

    pub fn write<T: Copy>(&mut self, addr: u32, val: T) {
        self.get_page(addr).write(addr, val)
    }

    pub fn allocate(&mut self, size: usize) {
        self.pages.clear();
        let pages_to_allocate = size / Self::PAGE_SIZE;
        for _ in 0..pages_to_allocate {
            self.pages.push(VramPage::default())
        }
    }

    pub fn map(&mut self, ptr: *mut u8, offset: usize, length: usize) {
        let pages_to_map = length / Self::PAGE_SIZE;
        for i in 0..pages_to_map {
            let index = (offset / Self::PAGE_SIZE) + i;
            self.pages[index].add_bank(unsafe { ptr.add(i * Self::PAGE_SIZE) })
        }
    }

    fn get_page(&mut self, mut addr: u32) -> &mut VramPage {
        addr &= 0xffffff;
        let region = (addr >> 20) & 0xf;
        let offset = addr - (region * 0x100000);
        let index = offset >> 12;
        // println!("{addr:x}, {region}, {offset:x}, {index}");
        // unsafe {
        //     &mut * self.pages.as_mut_ptr().add(index as usize)
        // }
        &mut self.pages[index as usize]
    }
}
