use log::{debug, error};

use crate::arm::coprocessor::Coprocessor;
use crate::bitfield;
use crate::core::arm9::memory::Arm9Memory;
use crate::util::Shared;

pub struct Arm9Coprocessor {
    memory: Shared<Arm9Memory>,

    control: Control,
    dtcm: [u8; 0x8000],
    itcm: [u8; 0x4000],
    dtcm_control: TcmControl,
    itcm_control: TcmControl,
}

impl Arm9Coprocessor {
    pub fn new(memory: &Shared<Arm9Memory>) -> Self {
        Self {
            memory: memory.clone(),
            control: Control(0),
            dtcm: [0; 0x8000],
            itcm: [0; 0x4000],
            dtcm_control: TcmControl(0),
            itcm_control: TcmControl(0),
        }
    }
}

impl Coprocessor for Arm9Coprocessor {
    fn read(&mut self, cn: u32, cm: u32, cp: u32) -> u32 {
        match (cn << 16) | (cm << 8) | cp {
            0x000001 => 0x0f0d2112, // chip id
            0x010000 => self.control.0,
            0x090100 => self.dtcm_control.0,
            0x090101 => self.itcm_control.0,
            _ => {
                error!("ARM9Coprocessor: handle register read c{cn}, c{cm}, c{cp}");
                0
            }
        }
    }

    fn write(&mut self, cn: u32, cm: u32, cp: u32, val: u32) {
        match (cn << 16) | (cm << 8) | cp {
            0x010000 => {
                self.control.0 = val;
                self.memory.dtcm.enable_reads =
                    self.control.dtcm_enable() && !self.control.dtcm_write_only();
                self.memory.dtcm.enable_writes = self.control.dtcm_enable();
                self.memory.itcm.enable_reads =
                    self.control.itcm_enable() && !self.control.itcm_write_only();
                self.memory.itcm.enable_writes = self.control.itcm_enable();
            }
            0x020000 => {}
            0x020001 => {}
            0x030000 => {}
            0x050002 => {}
            0x050003 => {}
            0x060000 => {}
            0x060100 => {}
            0x060200 => {}
            0x060300 => {}
            0x060400 => {}
            0x060500 => {}
            0x060600 => {}
            0x060700 => {}
            0x070500 => {}
            0x070501 => {}
            0x070600 => {}
            0x070601 => {}
            0x070602 => {}
            0x070a01 => {}
            0x070a02 => {}
            0x070a04 => {}
            0x070e01 => {}
            0x070e02 => {}
            0x090100 => {
                self.dtcm_control.0 = val;
                self.memory.dtcm.base = self.dtcm_control.base() << 12;
                self.memory.dtcm.limit = self.memory.dtcm.base + (512 << self.dtcm_control.size());
                debug!(
                    "ARM9Coprocessor: dtcm base = {:x}, limit = {:x}",
                    self.memory.dtcm.base, self.memory.dtcm.limit
                )
            }
            0x090101 => {
                self.itcm_control.0 = val;
                self.memory.itcm.base = 0;
                self.memory.itcm.limit = 512 << self.itcm_control.size();
                debug!(
                    "ARM9Coprocessor: itcm base = {:x}, limit = {:x}",
                    self.memory.itcm.base, self.memory.itcm.limit
                )
            }
            _ => error!("ARM9Coprocessor: handle register write c{cn}, c{cm}, c{cp} = {val:08x}"),
        }
    }

    fn get_exception_base(&self) -> u32 {
        if self.control.exception_vector() {
            0xffff0000
        } else {
            0x00000000
        }
    }
}

bitfield! {
    struct Control(u32) {
        mmu: bool => 0,
        alignment_faul: bool => 1,
        data_cache: bool => 2,
        write_buffer: bool => 3,
        exception_handling: bool => 4,
        faults_26bit: bool => 5,
        abort_model: bool => 6,
        endian: bool => 7,
        system_protection: bool => 8,
        rom_protection: bool => 9,
        // 10
        branch_prediction: bool => 11,
        instruction_cache: bool => 12,
        exception_vector: bool => 13,
        cache_replacement: bool => 14,
        pre_armv5: bool => 15,
        dtcm_enable: bool => 16,
        dtcm_write_only: bool => 17,
        itcm_enable: bool => 18,
        itcm_write_only: bool => 19,
        // 20 | 21
        unaligned_access: bool => 22,
        extended_page_table: bool => 23,
        // 24
        cpsr_on_exceptions: bool => 25,
        // 26
        fiq_behaviour: bool => 27,
        tex_remap: bool => 28,
        force_ap: bool => 29
        // 30 | 31
    }
}

bitfield! {
    struct TcmControl(u32) {
        // 0
        size: u32 => 1 | 5,
        // 6 | 11
        base: u32 => 12 | 31
    }
}
