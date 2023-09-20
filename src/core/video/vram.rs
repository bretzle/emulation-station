enum Bank {
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

pub struct Vram {
    pub bga: VramRegion,
}

impl Vram {
    pub fn new() -> Self {
        let mut vram = Self {
            bga: VramRegion::default(),
        };
        vram.reset(); // todo: reset all components in a better way
        vram
    }

    pub fn reset(&mut self) {
        self.bga.allocate(0x80000);

        self.reset_regions();
    }

    fn reset_regions(&mut self) {
        self.bga.reset();
    }

    pub fn read(&mut self, addr: u32) -> u32 {
        let region = (addr >> 20) & 0xf;
        match region {
            0x0 | 0x1 => todo!(), // self.bga.read(addr),
            0x2 | 0x3 => todo!(),
            0x4 | 0x5 => todo!(),
            0x6 | 0x7 => todo!(),
            _ => todo!(),
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

    pub fn read(&mut self, addr: u32) -> u32 {
        unsafe {
            let mut data = 0;
            for bank in &self.banks {
                data |= *bank.add((addr & Self::PAGE_MASK) as usize).cast::<u32>();
            }
            data
        }
    }

    pub fn write(&mut self, addr: u32, val: u32) {
        unsafe {
            for bank in &self.banks {
                *bank.add((addr & Self::PAGE_MASK) as usize).cast() = val;
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

    pub fn read(&mut self, addr: u32) -> u32 {
        self.get_page(addr).read(addr)
    }

    pub fn write(&mut self, addr: u32, val: u32) {
        self.get_page(addr).write(addr, val)
    }

    pub fn allocate(&mut self, size: usize) {
        self.pages.clear();
        let pages_to_allocate = size / Self::PAGE_SIZE;
        for _ in 0..pages_to_allocate {
            self.pages.push(VramPage::default())
        }
    }

    pub unsafe fn map(&mut self, ptr: *mut u8, offset: usize, length: usize) {
        let pages_to_map = length / Self::PAGE_SIZE;
        for i in 0..pages_to_map {
            let index = (offset / Self::PAGE_SIZE) + i;
            self.pages[index].add_bank(ptr.add(i + Self::PAGE_SIZE))
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
