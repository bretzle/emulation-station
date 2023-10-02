pub enum RegionAttributes {
    Read = 0b01,
    Write = 0b10,
    ReadWrite = 0b11,
}

pub struct PageTable<const N: usize> {
    read: Table<N>,
    write: Table<N>,
}

impl<const N: usize> PageTable<N> {
    pub fn new() -> Self {
        Self {
            read: Table::new(),
            write: Table::new(),
        }
    }

    pub fn map(&mut self, base: u32, end: u32, ptr: *mut u8, mask: u32, attributes: RegionAttributes) {
        match attributes {
            RegionAttributes::Read => self.read.map(base, end, ptr, mask),
            RegionAttributes::Write => self.write.map(base, end, ptr, mask),
            RegionAttributes::ReadWrite => {
                self.read.map(base, end, ptr, mask);
                self.write.map(base, end, ptr, mask);
            }
        }
    }

    pub fn unmap(&mut self, base: u32, end: u32, attributes: RegionAttributes) {
        match attributes {
            RegionAttributes::Read => self.read.unmap(base, end),
            RegionAttributes::Write => self.write.unmap(base, end),
            RegionAttributes::ReadWrite => {
                self.read.unmap(base, end);
                self.write.unmap(base, end);
            }
        }
    }

    pub fn read_pointer<T>(&self, addr: u32) -> *mut T {
        self.read.get_pointer(addr)
    }

    pub fn write_pointer<T>(&self, addr: u32) -> *mut T {
        self.write.get_pointer(addr)
    }
}

/// this class will be in the form of a 2 level page table, to save on space
/// since a large chunk of the 32-bit address space gets unused, it will be more memory efficient to do 2 levels
/// const generic specifies the number of bits per page
struct Table<const N: usize> {
    inner: Box<[Box<[*mut u8]>]>,
}

impl<const N: usize> Table<N> {
    const PAGE_SIZE: u32 = 1 << N;
    const PAGE_MASK: u32 = Self::PAGE_SIZE - 1;
    const L1_BITS: u32 = (32 - N as u32) / 2;
    const L1_SHIFT: u32 = 32 - Self::L1_BITS;
    const L1_SIZE: usize = 1 << Self::L1_BITS;
    const L1_MASK: usize = Self::L1_SIZE - 1;
    const L2_BITS: u32 = (32 - N as u32) / 2;
    const L2_SHIFT: u32 = 32 - Self::L1_BITS - Self::L2_BITS;
    const L2_SIZE: usize = 1 << Self::L2_BITS;
    const L2_MASK: usize = Self::L2_SIZE - 1;

    pub fn new() -> Self {
        Self {
            inner: vec![vec![0 as _; Self::L2_SIZE].into_boxed_slice(); Self::L1_SIZE].into_boxed_slice(),
        }
    }

    pub fn get_pointer<T>(&self, addr: u32) -> *mut T {
        let l1_entry = &self.inner[Self::get_l1_index(addr)];
        let l2_entry = l1_entry[Self::get_l2_index(addr)];

        if l2_entry.is_null() {
            return 0 as _;
        }

        let offset = addr & Self::PAGE_MASK;
        unsafe { l2_entry.add(offset as usize).cast() }
    }

    pub fn map(&mut self, base: u32, end: u32, ptr: *mut u8, mask: u32) {
        for addr in (base..end).step_by(Self::PAGE_SIZE as usize) {
            let l1_entry = &mut self.inner[Self::get_l1_index(addr)];
            let l2_entry = &mut l1_entry[Self::get_l2_index(addr)];
            let offset = addr & mask;
            *l2_entry = unsafe { ptr.add(offset as usize) }
        }
    }

    pub fn unmap(&mut self, base: u32, end: u32) {
        for addr in (base..end).step_by(Self::PAGE_SIZE as _) {
            let l1_entry = &mut self.inner[Self::get_l1_index(addr)];
            let l2_entry = &mut l1_entry[Self::get_l2_index(addr)];
            *l2_entry = std::ptr::null_mut();
        }
    }

    const fn get_l1_index(addr: u32) -> usize {
        addr as usize >> Self::L1_SHIFT
    }

    const fn get_l2_index(addr: u32) -> usize {
        (addr as usize >> Self::L2_SHIFT) & Self::L2_MASK
    }
}
