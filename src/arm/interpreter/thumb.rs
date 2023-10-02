use crate::arm::cpu::{Arch, Cpu};
use crate::arm::interpreter::instructions::*;
use crate::arm::state::{Bank, Mode, GPR};

#[allow(dead_code)]
impl Cpu {
    pub(in crate::arm) fn thumb_alu_immediate(&mut self, instruction: u32) {
        let ThumbALUImmediate { imm, rd, opcode } = ThumbALUImmediate::decode(instruction);
        match opcode {
            ThumbALUImmediateOp::MOV => {
                self.state.gpr[rd as usize] = imm;
                self.state.cpsr.set_n(false);
                self.state.cpsr.set_z(imm == 0);
            }
            ThumbALUImmediateOp::CMP => self.alu_cmp(self.state.gpr[rd as usize], imm),
            ThumbALUImmediateOp::ADD => self.state.gpr[rd as usize] = self.alu_add(self.state.gpr[rd as usize], imm, true),
            ThumbALUImmediateOp::SUB => self.state.gpr[rd as usize] = self.alu_sub(self.state.gpr[rd as usize], imm, true),
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_branch_link_offset(&mut self, instruction: u32) {
        let ThumbBranchLinkOffset { offset } = ThumbBranchLinkOffset::decode(instruction);
        let next_instruction_addr = self.state.gpr[15] - 2;
        self.state.gpr[15] = (self.state.gpr[14] + offset) & !0x1;
        self.state.gpr[14] = next_instruction_addr | 0x1;
        self.thumb_flush_pipeline();
    }

    pub(in crate::arm) fn thumb_branch_link_setup(&mut self, instruction: u32) {
        let ThumbBranchLinkSetup { imm } = ThumbBranchLinkSetup::decode(instruction);
        self.state.gpr[14] = self.state.gpr[15] + imm;
        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_branch_link_exchange_offset(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return;
        }

        let ThumbBranchLinkExchangeOffset { offset } = ThumbBranchLinkExchangeOffset::decode(instruction);
        let next_instruction_addr = self.state.gpr[15] - 2;
        self.state.gpr[15] = (self.state.gpr[14] + offset) & !0x3;
        self.state.gpr[14] = next_instruction_addr | 0x1;
        self.state.cpsr.set_thumb(false);
        self.arm_flush_pipeline();
    }

    pub(in crate::arm) fn thumb_branch(&mut self, instruction: u32) {
        let ThumbBranch { offset } = ThumbBranch::decode(instruction);
        self.state.gpr[15] += offset;
        self.thumb_flush_pipeline();
    }

    pub(in crate::arm) fn thumb_push_pop(&mut self, instruction: u32) {
        let ThumbPushPop { rlist, pclr, pop } = ThumbPushPop::decode(instruction);
        let mut addr = self.state.gpr[13];

        if pop {
            for i in 0..8 {
                if rlist & (1 << i) != 0 {
                    self.state.gpr[i] = self.memory.read_word(addr);
                    addr += 4;
                }
            }

            if pclr {
                self.state.gpr[15] = self.memory.read_word(addr);
                self.state.gpr[13] = addr + 4;

                if (self.arch == Arch::ARMv4) || (self.state.gpr[15] & 0x1 != 0) {
                    self.state.gpr[15] &= !0x1;
                    self.thumb_flush_pipeline();
                } else {
                    self.state.cpsr.set_thumb(false);
                    self.state.gpr[15] &= !0x3;
                    self.arm_flush_pipeline();
                }
            } else {
                self.state.gpr[15] += 2;
                self.state.gpr[13] = addr;
            }
        } else {
            for i in 0..8 {
                if rlist & (1 << i) != 0 {
                    addr -= 4;
                }
            }

            if pclr {
                addr -= 4;
            }

            self.state.gpr[13] = addr;

            for i in 0..8 {
                if rlist & (1 << i) != 0 {
                    self.memory.write_word(addr, self.state.gpr[i]);
                    addr += 4;
                }
            }

            if pclr {
                self.memory.write_word(addr, self.state.gpr[14]);
            }

            self.state.gpr[15] += 2;
        }
    }

    pub(in crate::arm) fn thumb_data_processing_register(&mut self, instruction: u32) {
        let ThumbDataProcessingRegister { rd, rs, opcode } = ThumbDataProcessingRegister::decode(instruction);
        let mut carry = self.state.cpsr.c();

        match opcode {
            ThumbOpcode::AND => self.state.gpr[rd as usize] = self.alu_and(self.state.gpr[rd as usize], self.state.gpr[rs as usize], true),
            ThumbOpcode::EOR => self.state.gpr[rd as usize] = self.alu_eor(self.state.gpr[rd as usize], self.state.gpr[rs as usize], true),
            ThumbOpcode::LSL => {
                self.state.gpr[rd as usize] = self.alu_lsl(self.state.gpr[rd as usize], self.state.gpr[rs as usize] & 0xff, &mut carry);
                self.state.cpsr.set_c(carry);
                self.set_nz(self.state.gpr[rd as usize])
            }
            ThumbOpcode::LSR => {
                self.state.gpr[rd as usize] =
                    self.alu_lsr(self.state.gpr[rd as usize], self.state.gpr[rs as usize] & 0xff, &mut carry, false);
                self.state.cpsr.set_c(carry);
                self.set_nz(self.state.gpr[rd as usize])
            }
            ThumbOpcode::ASR => {
                self.state.gpr[rd as usize] =
                    self.alu_asr(self.state.gpr[rd as usize], self.state.gpr[rs as usize] & 0xff, &mut carry, false);
                self.state.cpsr.set_c(carry);
                self.set_nz(self.state.gpr[rd as usize])
            }
            ThumbOpcode::ADC => self.state.gpr[rd as usize] = self.alu_adc(self.state.gpr[rd as usize], self.state.gpr[rs as usize], true),
            ThumbOpcode::SBC => self.state.gpr[rd as usize] = self.alu_sbc(self.state.gpr[rd as usize], self.state.gpr[rs as usize], true),
            ThumbOpcode::ROR => {
                self.state.gpr[rd as usize] =
                    self.alu_ror(self.state.gpr[rd as usize], self.state.gpr[rs as usize] & 0xff, &mut carry, false);
                self.state.cpsr.set_c(carry);
                self.set_nz(self.state.gpr[rd as usize])
            }
            ThumbOpcode::TST => self.alu_tst(self.state.gpr[rd as usize], self.state.gpr[rs as usize]),
            ThumbOpcode::NEG => self.state.gpr[rd as usize] = self.alu_sub(0, self.state.gpr[rs as usize], true),
            ThumbOpcode::CMP => self.alu_cmp(self.state.gpr[rd as usize], self.state.gpr[rs as usize]),
            ThumbOpcode::CMN => self.alu_cmn(self.state.gpr[rd as usize], self.state.gpr[rs as usize]),
            ThumbOpcode::ORR => self.state.gpr[rd as usize] = self.alu_orr(self.state.gpr[rd as usize], self.state.gpr[rs as usize], true),
            ThumbOpcode::MUL => {
                self.state.gpr[rd as usize] *= self.state.gpr[rs as usize];
                self.set_nz(self.state.gpr[rd as usize])
            }
            ThumbOpcode::BIC => self.state.gpr[rd as usize] = self.alu_bic(self.state.gpr[rd as usize], self.state.gpr[rs as usize], true),
            ThumbOpcode::MVN => self.state.gpr[rd as usize] = self.alu_mvn(self.state.gpr[rs as usize], true),
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_special_data_processing(&mut self, instruction: u32) {
        let ThumbSpecialDataProcessing { rd, rs, opcode } = ThumbSpecialDataProcessing::decode(instruction);
        match opcode {
            SpecialOpcode::ADD => {
                self.state.gpr[rd as usize] += self.state.gpr[rs as usize];
                if rd == GPR::PC {
                    self.thumb_flush_pipeline()
                } else {
                    self.state.gpr[15] += 2;
                }
            }
            SpecialOpcode::CMP => {
                self.alu_cmp(self.state.gpr[rd as usize], self.state.gpr[rs as usize]);
                self.state.gpr[15] += 2;
            }
            SpecialOpcode::MOV => {
                self.state.gpr[rd as usize] = self.state.gpr[rs as usize];
                if rd == GPR::PC {
                    self.thumb_flush_pipeline()
                } else {
                    self.state.gpr[15] += 2;
                }
            }
        }
    }

    pub(in crate::arm) fn thumb_branch_link_exchange(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return;
        }

        let ThumbBranchLinkExchange { rm } = ThumbBranchLinkExchange::decode(instruction);
        let next_instruction_addr = self.state.gpr[15] - 2;
        self.state.gpr[14] = next_instruction_addr | 0x1;

        if self.state.gpr[rm as usize] & 0x1 != 0 {
            self.state.gpr[15] = self.state.gpr[rm as usize] & !0x1;
            self.thumb_flush_pipeline();
        } else {
            self.state.cpsr.set_thumb(false);
            self.state.gpr[15] = self.state.gpr[rm as usize] & !0x3;
            self.arm_flush_pipeline();
        }
    }

    pub(in crate::arm) fn thumb_branch_exchange(&mut self, instruction: u32) {
        let ThumbBranchExchange { rm } = ThumbBranchExchange::decode(instruction);
        if self.state.gpr[rm as usize] & 0x1 != 0 {
            self.state.gpr[15] = self.state.gpr[rm as usize] & !0x1;
            self.thumb_flush_pipeline();
        } else {
            self.state.cpsr.set_thumb(false);
            self.state.gpr[15] = self.state.gpr[rm as usize] & !0x3;
            self.arm_flush_pipeline();
        }
    }

    pub(in crate::arm) fn thumb_load_store_register_offset(&mut self, instruction: u32) {
        let ThumbLoadStoreRegisterOffset { rd, rn, rm, opcode } = ThumbLoadStoreRegisterOffset::decode(instruction);
        let addr = self.state.gpr[rn as usize] + self.state.gpr[rm as usize];
        match opcode {
            LoadStoreRegisterOpcode::STR => self.memory.write_word(addr, self.state.gpr[rd as usize]),
            LoadStoreRegisterOpcode::STRB => self.memory.write_byte(addr, self.state.gpr[rd as usize] as u8),
            LoadStoreRegisterOpcode::LDR => self.state.gpr[rd as usize] = self.read_word_rotate(addr),
            LoadStoreRegisterOpcode::LDRB => self.state.gpr[rd as usize] = self.memory.read_byte(addr) as u32,
        }
    }

    pub(in crate::arm) fn thumb_load_store_signed(&mut self, instruction: u32) {
        let ThumbLoadStoreSigned { rd, rn, rm, opcode } = ThumbLoadStoreSigned::decode(instruction);
        let addr = self.state.gpr[rn as usize] + self.state.gpr[rm as usize];
        match opcode {
            LoadStoreSignedOpcode::STRH => self.memory.write_half(addr, self.state.gpr[rd as usize] as u16),
            LoadStoreSignedOpcode::LDRSB => self.state.gpr[rd as usize] = sign_extend::<8>(self.memory.read_byte(addr) as u32),
            LoadStoreSignedOpcode::LDRH => self.state.gpr[rd as usize] = self.memory.read_half(addr) as u32,
            LoadStoreSignedOpcode::LDRSH => self.state.gpr[rd as usize] = sign_extend::<16>(self.memory.read_half(addr) as u32),
        }
        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_load_pc(&mut self, instruction: u32) {
        let ThumbLoadPC { imm, rd } = ThumbLoadPC::decode(instruction);
        let addr = (self.state.gpr[15] & !0x2) + imm;
        self.state.gpr[rd as usize] = self.memory.read_word(addr);
        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_load_store_sp_relative(&mut self, instruction: u32) {
        let ThumbLoadStoreSPRelative { imm, rd, load } = ThumbLoadStoreSPRelative::decode(instruction);
        let addr = self.state.gpr[13] + (imm << 2);
        if load {
            self.state.gpr[rd as usize] = self.read_word_rotate(addr);
        } else {
            self.memory.write_word(addr, self.state.gpr[rd as usize]);
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_load_store_halfword(&mut self, instruction: u32) {
        let ThumbLoadStoreHalfword { rd, rn, imm, load } = ThumbLoadStoreHalfword::decode(instruction);
        let addr = self.state.gpr[rn as usize] + (imm << 1);
        if load {
            self.state.gpr[rd as usize] = self.memory.read_half(addr) as u32;
        } else {
            self.memory.write_half(addr, self.state.gpr[rd as usize] as u16);
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_add_subtract(&mut self, instruction: u32) {
        let ThumbAddSubtract { rd, rs, rn, sub, imm } = ThumbAddSubtract::decode(instruction);
        let lhs = self.state.gpr[rs as usize];
        let rhs = if imm { rn as u32 } else { self.state.gpr[rn as usize] };

        if sub {
            self.state.gpr[rd as usize] = self.alu_sub(lhs, rhs, true);
        } else {
            self.state.gpr[rd as usize] = self.alu_add(lhs, rhs, true);
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_shift_immediate(&mut self, instruction: u32) {
        let ThumbShiftImmediate {
            rd,
            rs,
            amount,
            shift_type,
        } = ThumbShiftImmediate::decode(instruction);
        let (result, carry) = self.barrel_shifter(self.state.gpr[rs as usize], shift_type, amount, true);
        self.state.gpr[rd as usize] = result;
        if let Some(carry) = carry {
            self.state.cpsr.set_c(carry)
        }

        self.set_nz(self.state.gpr[rd as usize]);
        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_software_interrupt(&mut self, _instruction: u32) {
        self.state.spsr_at(Bank::SVC).0 = self.state.cpsr.0;
        self.switch_mode(Mode::Supervisor);

        self.state.cpsr.set_thumb(true);
        self.state.cpsr.set_i(true);
        self.state.gpr[14] = self.state.gpr[15] - 2;
        self.state.gpr[15] = self.coprocessor.get_exception_base() + 0x08;
        self.arm_flush_pipeline();
    }

    pub(in crate::arm) fn thumb_branch_conditional(&mut self, instruction: u32) {
        let ThumbBranchConditional { condition, offset } = ThumbBranchConditional::decode(instruction);
        if self.evaluate_cond(condition) {
            self.state.gpr[15] += offset;
            self.thumb_flush_pipeline();
        } else {
            self.state.gpr[15] += 2;
        }
    }

    pub(in crate::arm) fn thumb_load_store_multiple(&mut self, instruction: u32) {
        let ThumbLoadStoreMultiple { rlist, rn, load } = ThumbLoadStoreMultiple::decode(instruction);
        let mut addr = self.state.gpr[rn as usize];

        if rlist == 0 {
            self.state.gpr[15] += 2;

            if self.arch == Arch::ARMv4 {
                if load {
                    self.state.gpr[15] = self.memory.read_word(addr);
                } else {
                    self.memory.write_word(addr, self.state.gpr[15]);
                }
            }

            self.state.gpr[rn as usize] = addr + 0x40;
            return;
        }

        if load {
            for i in 0..8 {
                if rlist & (1 << i) != 0 {
                    self.state.gpr[i] = self.memory.read_word(addr);
                    addr += 4;
                }
            }

            // TODO: sort out edgecases with writeback
            if !rlist & (1 << rn as usize) != 0 {
                self.state.gpr[rn as usize] = addr;
            }
        } else {
            for i in 0..8 {
                if rlist & (1 << i) != 0 {
                    self.memory.write_word(addr, self.state.gpr[i]);
                    addr += 4;
                }
            }

            self.state.gpr[rn as usize] = addr;
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_load_store_immediate(&mut self, instruction: u32) {
        let ThumbLoadStoreImmediate { rd, rn, imm, opcode } = ThumbLoadStoreImmediate::decode(instruction);
        match opcode {
            LoadStoreOpcode::STR => self
                .memory
                .write_word(self.state.gpr[rn as usize] + (imm << 2), self.state.gpr[rd as usize]),
            LoadStoreOpcode::LDR => {
                self.state.gpr[rd as usize] = self.read_word_rotate(self.state.gpr[rn as usize] + (imm << 2));
            }
            LoadStoreOpcode::STRB => {
                self.memory
                    .write_byte(self.state.gpr[rn as usize] + imm, self.state.gpr[rd as usize] as u8);
            }
            LoadStoreOpcode::LDRB => {
                self.state.gpr[rd as usize] = self.memory.read_byte(self.state.gpr[rn as usize] + imm) as u32;
            }
        }

        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_add_sp_pc(&mut self, instruction: u32) {
        let ThumbAddSPPC { imm, rd, sp } = ThumbAddSPPC::decode(instruction);
        if sp {
            self.state.gpr[rd as usize] = self.state.gpr[13] + imm;
        } else {
            self.state.gpr[rd as usize] = (self.state.gpr[15] & !0x2) + imm;
        }
        self.state.gpr[15] += 2;
    }

    pub(in crate::arm) fn thumb_adjust_stack_pointer(&mut self, instruction: u32) {
        let ThumbAdjustStackPointer { imm, sub } = ThumbAdjustStackPointer::decode(instruction);
        if sub {
            self.state.gpr[13] -= imm;
        } else {
            self.state.gpr[13] += imm;
        }

        self.state.gpr[15] += 2;
    }
}
