use crate::arm::coprocessor::Coprocessor;

pub struct Arm7Coprocessor;

impl Coprocessor for Arm7Coprocessor {
    fn read(&mut self, cn: u32, cm: u32, cp: u32) -> u32 {
        unimplemented!()
    }

    fn write(&mut self, cn: u32, cm: u32, cp: u32, val: u32) {
        unimplemented!()
    }

    fn get_exception_base(&self) -> u32 {
        0
    }
}