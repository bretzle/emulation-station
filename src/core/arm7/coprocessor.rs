use crate::arm::coprocessor::Coprocessor;

pub struct Arm7Coprocessor;

impl Coprocessor for Arm7Coprocessor {
    fn read(&mut self, _cn: u32, _cm: u32, _cp: u32) -> u32 {
        unimplemented!()
    }

    fn write(&mut self, _cn: u32, _cm: u32, _cp: u32, _val: u32) {
        unimplemented!()
    }

    fn get_exception_base(&self) -> u32 {
        0
    }
}
