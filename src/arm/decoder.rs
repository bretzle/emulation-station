use crate::arm::coprocessor::Coprocessor;
use crate::arm::cpu::Cpu;
use crate::arm::memory::Memory;

type Handler = fn(&mut Cpu, u32);

struct Info {
    handler: Handler,
    mask: u32,
    value: u32,
}

pub struct Decoder {
    arm_lut: [Handler; 4096],
    thumb_lut: [Handler; 1024],
    arm_list: Vec<Info>,
    thumb_list: Vec<Info>,
}

impl Decoder {
    pub fn new() -> Self {
        let mut decoder = Self {
            arm_lut: [Cpu::illegal_instruction; 4096],
            thumb_lut: [Cpu::illegal_instruction; 1024],
            arm_list: vec![],
            thumb_list: vec![],
        };

        decoder.register_arm("101xxxxxxxxx", Cpu::arm_branch_link_maybe_exchange);
        decoder.register_arm("000100100001", Cpu::arm_branch_exchange);
        decoder.register_arm("000101100001", Cpu::arm_count_leading_zeroes);
        decoder.register_arm("000100100011", Cpu::arm_branch_link_exchange_register);
        decoder.register_arm("00010x001001", Cpu::arm_single_data_swap);
        decoder.register_arm("000000xx1001", Cpu::arm_multiply);
        decoder.register_arm("00010xx00101", Cpu::arm_saturating_add_subtract);
        decoder.register_arm("00001xxx1001", Cpu::arm_multiply_long);
        decoder.register_arm("000xxxxx1xx1", Cpu::arm_halfword_data_transfer);
        decoder.register_arm("00010x000000", Cpu::arm_status_load);
        decoder.register_arm("00010x100000", Cpu::arm_status_store_register);
        decoder.register_arm("00110x10xxxx", Cpu::arm_status_store_immediate);
        decoder.register_arm("100xxxxxxxxx", Cpu::arm_block_data_transfer);
        decoder.register_arm("01xxxxxxxxxx", Cpu::arm_single_data_transfer);
        decoder.register_arm("00xxxxxxxxxx", Cpu::arm_data_processing);
        decoder.register_arm("1110xxxxxxx1", Cpu::arm_coprocessor_register_transfer);
        decoder.register_arm("1111xxxxxxxx", Cpu::arm_software_interrupt);
        decoder.register_arm("000101001xx0", Cpu::arm_signed_multiply_accumulate_long);
        decoder.register_arm("000100101xx0", Cpu::arm_signed_multiply_word);
        decoder.register_arm("00010xx01xx0", Cpu::arm_signed_multiply);
        decoder.register_arm("000100100111", Cpu::arm_breakpoint);

        decoder
            .arm_list
            .sort_by(|a, b| a.mask.count_ones().cmp(&b.mask.count_ones()));

        for i in 0..decoder.arm_lut.len() as u32 {
            for info in &decoder.arm_list {
                if (i & info.mask) == info.value {
                    decoder.arm_lut[i as usize] = info.handler;
                }
            }
        }

        decoder.register_thumb("001xxxxxxx", Cpu::thumb_alu_immediate);
        decoder.register_thumb("11111xxxxx", Cpu::thumb_branch_link_offset);
        decoder.register_thumb("11110xxxxx", Cpu::thumb_branch_link_setup);
        decoder.register_thumb("11101xxxxx", Cpu::thumb_branch_link_exchange_offset);
        decoder.register_thumb("11100xxxxx", Cpu::thumb_branch);
        decoder.register_thumb("1011x10xxx", Cpu::thumb_push_pop);
        decoder.register_thumb("010000xxxx", Cpu::thumb_data_processing_register);
        decoder.register_thumb("010001xxxx", Cpu::thumb_special_data_processing);
        decoder.register_thumb("010001111x", Cpu::thumb_branch_link_exchange);
        decoder.register_thumb("010001110x", Cpu::thumb_branch_exchange);
        decoder.register_thumb("0101xx0xxx", Cpu::thumb_load_store_register_offset);
        decoder.register_thumb("0101xx1xxx", Cpu::thumb_load_store_signed);
        decoder.register_thumb("01001xxxxx", Cpu::thumb_load_pc);
        decoder.register_thumb("1001xxxxxx", Cpu::thumb_load_store_sp_relative);
        decoder.register_thumb("1000xxxxxx", Cpu::thumb_load_store_halfword);
        decoder.register_thumb("00011xxxxx", Cpu::thumb_add_subtract);
        decoder.register_thumb("000xxxxxxx", Cpu::thumb_shift_immediate);
        decoder.register_thumb("11011111xx", Cpu::thumb_software_interrupt);
        decoder.register_thumb("1101xxxxxx", Cpu::thumb_branch_conditional);
        decoder.register_thumb("1100xxxxxx", Cpu::thumb_load_store_multiple);
        decoder.register_thumb("011xxxxxxx", Cpu::thumb_load_store_immediate);
        decoder.register_thumb("1010xxxxxx", Cpu::thumb_add_sp_pc);
        decoder.register_thumb("10110000xx", Cpu::thumb_adjust_stack_pointer);

        decoder
            .thumb_list
            .sort_by(|a, b| a.mask.count_ones().cmp(&b.mask.count_ones()));

        for i in 0..decoder.thumb_lut.len() as u32 {
            for info in &decoder.thumb_list {
                if (i & info.mask) == info.value {
                    decoder.thumb_lut[i as usize] = info.handler;
                }
            }
        }

        decoder
    }

    fn register_arm(&mut self, pattern: &str, handler: Handler) {
        let mask = mask::<32>(pattern);
        let value = value::<32>(pattern);
        self.arm_list.push(Info {
            handler,
            mask,
            value,
        });
    }

    fn register_thumb(&mut self, pattern: &str, handler: Handler) {
        let mask = mask::<16>(pattern);
        let value = value::<16>(pattern);
        self.thumb_list.push(Info {
            handler,
            mask,
            value,
        });
    }

    #[inline]
    pub fn decode_arm(&self, instruction: u32) -> Handler {
        let idx = ((instruction >> 16) & 0xff0) | ((instruction >> 4) & 0xf);
        self.arm_lut[idx as usize]
    }

    #[inline]
    pub fn decode_thumb(&self, instruction: u32) -> Handler {
        let idx = instruction >> 6;
        self.thumb_lut[idx as usize]
    }
}

fn mask<const BITS: usize>(pattern: &str) -> u32 {
    let mut res = 0;

    for (i, c) in pattern.chars().enumerate() {
        if c == '0' || c == '1' {
            res |= 1 << (BITS - i - 1);
        }
    }

    res >> (BITS - pattern.len())
}

fn value<const BITS: usize>(pattern: &str) -> u32 {
    let mut res = 0;

    for (i, c) in pattern.chars().enumerate() {
        if c == '1' {
            res |= 1 << (BITS - i - 1);
        }
    }

    res >> (BITS - pattern.len())
}
