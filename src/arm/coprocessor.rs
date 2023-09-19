pub trait Coprocessor {
    fn read(&mut self, cn: u32, cm: u32, cp: u32) -> u32;
    fn write(&mut self, cn: u32, cm: u32, cp: u32, val: u32);
    fn get_exception_base(&self) -> u32;
}

pub struct Tcm {
    pub data: *mut u8,
    pub mask: u32,

    pub enable_reads: bool,
    pub enable_writes: bool,
    pub base: u32,
    pub limit: u32,
}

impl Default for Tcm {
    fn default() -> Self {
        Self {
            data: 0 as _,
            mask: 0,
            enable_reads: false,
            enable_writes: false,
            base: 0,
            limit: 0,
        }
    }
}
