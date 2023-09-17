use crate::arm::coprocessor::Coprocessor;

pub struct Arm9Coprocessor {}

impl Arm9Coprocessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Coprocessor for Arm9Coprocessor {}
