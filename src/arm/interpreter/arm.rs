use log::{error, warn};

use crate::arm::coprocessor::Coprocessor;
use crate::arm::cpu::{Arch, Cpu};
use crate::arm::interpreter::instructions::*;
use crate::arm::memory::Memory;
use crate::arm::state::{GPR, Mode};

#[allow(dead_code)]
impl<M: Memory, C: Coprocessor> Cpu<M, C> {
    pub(in crate::arm) fn arm_branch_link_maybe_exchange(&mut self, instruction: u32) {
        if (instruction & 0xf0000000) != 0xf0000000 {
            self.arm_branch_link(instruction);
        } else {
            self.arm_branch_link_exchange(instruction);
        }
    }

    pub(in crate::arm) fn arm_branch_exchange(&mut self, _: u32) {
        todo!()
    }

    fn arm_branch_link(&mut self, instruction: u32) {
        let ArmBranchLink {
            link,
            offset,
            condition,
        } = ArmBranchLink::decode(instruction);
        if link {
            self.state.gpr[14] = self.state.gpr[15] - 4;
        }

        self.state.gpr[15] += offset;
        self.arm_flush_pipeline();
    }

    fn arm_branch_link_exchange(&mut self, instruction: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_count_leading_zeroes(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_branch_link_exchange_register(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_single_data_swap(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_multiply(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_saturating_add_subtract(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_multiply_long(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_halfword_data_transfer(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_status_load(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_status_store_register(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_status_store_immediate(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_block_data_transfer(&mut self, instruction: u32) {
        let ArmBlockDataTransfer {
            mut rlist,
            mut r15_in_rlist,
            load,
            writeback,
            psr,
            up,
            mut pre,
            rn,
        } = ArmBlockDataTransfer::decode(instruction);

        let mut addr = self.state.gpr[rn as usize];
        let old_mode = self.state.cpsr.mode();
        let mut first = 0;
        let mut bytes = 0;
        let mut new_base = 0;

        if rlist != 0 {
            for i in (0..15).rev() {
                if rlist & (1 << i) != 0 {
                    first = i;
                    bytes += 4;
                }
            }
        } else {
            bytes = 0x40;
            if self.arch == Arch::ARMv4 {
                rlist = 1 << 15;
                r15_in_rlist = true;
            }
        }

        if up {
            new_base = addr + bytes;
        } else {
            pre = !pre;
            addr -= bytes;
            new_base = addr;
        }

        self.state.gpr[15] += 4;

        if writeback & !load {
            if self.arch == Arch::ARMv4 && first != rn as _ {
                self.state.gpr[rn as usize] = new_base;
            }
        }

        let user_switch_mode = psr && (!load || !r15_in_rlist);
        if user_switch_mode {
            self.switch_mode(Mode::User);
        }

        for i in first..16 {
            if !(rlist & (1 << i) != 0) {
                continue
            }

            if pre {
                addr += 4;
            }

            if load {
                self.state.gpr[i] = self.memory.read_word(addr);
            } else {
                self.memory.write_word(addr, self.state.gpr[i]);
            }

            if !pre {
                addr += 4;
            }
        }

        if writeback {
            if load {
                if self.arch == Arch::ARMv5 {
                    if (rlist == (1 << rn as u16)) || !((rlist >> rn as u16) == 1) {
                        self.state.gpr[rn as usize] = new_base;
                    }
                } else {
                    if !(rlist & (1 << rn as u16) != 0) {
                        self.state.gpr[rn as usize] = new_base;
                    }
                }
            } else {
                self.state.gpr[rn as usize] = new_base;
            }
        }

        if user_switch_mode {
            self.switch_mode(old_mode);
            if load && r15_in_rlist {
                todo!("Interpreter: handle loading into r15 in user mode")
            }
        }

        if load && r15_in_rlist {
            if self.arch == Arch::ARMv5 && self.state.gpr[15] & 1 != 0 {
                self.state.cpsr.set_thumb(true);
                self.thumb_flush_pipeline();
            } else {
                self.arm_flush_pipeline();
            }
        }
    }

    pub(in crate::arm) fn arm_single_data_transfer(&mut self, instruction: u32) {
        let opcode = ArmSingleDataTransfer::decode(instruction);
        let mut addr = self.state.gpr[opcode.rn as usize];
        let do_writeback = !opcode.load || opcode.rd != opcode.rn;

        let mut op2 = match opcode.rhs {
            ArmSingleDataTransferRhs::Imm(imm) => imm,
            ArmSingleDataTransferRhs::Reg { .. } => todo!(),
        };

        if !opcode.up {
            op2 *= u32::MAX
        }

        if opcode.pre {
            addr += op2;
        }

        self.state.gpr[15] += 4;

        if opcode.load {
            if opcode.byte {
                self.state.gpr[opcode.rd as usize] = todo!(); //read byte
            } else {
                self.state.gpr[opcode.rd as usize] = self.read_word_rotate(addr);
            }
        } else {
            if opcode.byte {
                todo!() // write byte
            } else {
                todo!() // write word
            }
        }

        if do_writeback {
            if !opcode.pre {
                self.state.gpr[opcode.rn as usize] += op2;
            } else if opcode.writeback {
                self.state.gpr[opcode.rn as usize] = addr;
            }
        }

        if opcode.load && opcode.rd == GPR::PC {
            if self.arch == Arch::ARMv5 && self.state.gpr[15] & 1 != 0 {
                todo!()
            } else {
                todo!()
            }
        }
    }

    pub(in crate::arm) fn arm_data_processing(&mut self, instruction: u32) {
        let ArmDataProcessing {
            set_flags,
            rd,
            rn,
            opcode,
            condition,
            rhs,
        } = ArmDataProcessing::decode(instruction);

        let mut op1 = self.state.gpr[rn as usize];
        let mut op2 = 0;

        let carry_done_in_opcode = matches!(opcode, Opcode::ADC | Opcode::SBC | Opcode::RSC);

        let set_carry = set_flags && !carry_done_in_opcode;

        match rhs {
            ArmDataProcessingRhs::Imm { shift, rotated } => {
                op2 = rotated;
                if set_carry && shift != 0 {
                    self.state.cpsr.set_c(op2 >> 31 != 0);
                }
            }
            ArmDataProcessingRhs::Reg {
                rm,
                shift_type,
                amount,
            } => {
                let mut amt = 0;
                let mut src = self.state.gpr[rm as usize];

                match amount {
                    ArmDataProcessingAmount::Rs(rs) => {
                        amt = self.state.gpr[rs as usize] & 0xff;

                        if rn == GPR::PC {
                            op1 += 4;
                        }

                        if rm == GPR::PC {
                            src += 4;
                        }
                    }
                    ArmDataProcessingAmount::Imm(val) => amt = val as _,
                }

                let imm = matches!(amount, ArmDataProcessingAmount::Imm(_));
                let (result, carry) = self.barrel_shifter(src, shift_type, amt, imm);
                op2 = result;
                if set_carry {
                    if let Some(carry) = carry {
                        self.state.cpsr.set_c(carry)
                    }
                }
            }
        }

        match opcode {
            Opcode::AND => self.state.gpr[rd as usize] = self.alu_and(op1, op2, set_flags),
            Opcode::EOR => self.state.gpr[rd as usize] = self.alu_eor(op1, op2, set_flags),
            Opcode::SUB => self.state.gpr[rd as usize] = self.alu_sub(op1, op2, set_flags),
            Opcode::RSB => self.state.gpr[rd as usize] = self.alu_sub(op2, op1, set_flags),
            Opcode::ADD => self.state.gpr[rd as usize] = self.alu_add(op1, op2, set_flags),
            Opcode::ADC => self.state.gpr[rd as usize] = self.alu_adc(op1, op2, set_flags),
            Opcode::SBC => self.state.gpr[rd as usize] = self.alu_sbc(op1, op2, set_flags),
            Opcode::RSC => self.state.gpr[rd as usize] = self.alu_sbc(op2, op1, set_flags),
            Opcode::TST => self.alu_tst(op1, op2),
            Opcode::TEQ => self.alu_teq(op1, op2),
            Opcode::CMP => self.alu_cmp(op1, op2),
            Opcode::CMN => self.alu_cmn(op1, op2),
            Opcode::ORR => self.state.gpr[rd as usize] = self.alu_orr(op1, op2, set_flags),
            Opcode::MOV => self.state.gpr[rd as usize] = self.alu_mov(op2, set_flags),
            Opcode::BIC => self.state.gpr[rd as usize] = self.alu_bic(op1, op2, set_flags),
            Opcode::MVN => self.state.gpr[rd as usize] = self.alu_mvn(op2, set_flags),
        }

        if rd == GPR::PC {
            if set_flags {
                let spsr = *self.state.spsr();
                self.switch_mode(spsr.mode());
                self.state.cpsr = spsr;
            }

            if !matches!(
                opcode,
                Opcode::TST | Opcode::TEQ | Opcode::CMP | Opcode::CMN
            ) {
                if self.state.cpsr.thumb() {
                    self.thumb_flush_pipeline();
                } else {
                    self.arm_flush_pipeline();
                }
            }
        } else {
            self.state.gpr[15] += 4;
        }
    }

    pub(in crate::arm) fn arm_coprocessor_register_transfer(&mut self, instruction: u32) {
        let opcode = ArmCoprocessorRegisterTransfer::decode(instruction);

        // TODO: handle this in a nicer way
        if self.arch == Arch::ARMv4 && opcode.cp == 14 {
            warn!("Interpreter: mrc cp14 on arm7");
            return;
        } else if (self.arch == Arch::ARMv4 && opcode.cp == 15)
            || (self.arch == Arch::ARMv5 && opcode.cp == 14)
        {
            self.undefined_exception();
            return;
        }

        if opcode.rd == GPR::PC {
            error!("Interpreter: handle rd == 15 in arm_coprocessor_register_transfer");
        }

        if opcode.load {
            self.state.gpr[opcode.rd as usize] =
                self.coprocessor
                    .read(opcode.crn as _, opcode.crm as _, opcode.cp as _);
        } else {
            self.coprocessor.write(
                opcode.crn as _,
                opcode.crm as _,
                opcode.cp as _,
                self.state.gpr[opcode.rd as usize],
            );
        }

        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_software_interrupt(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_signed_multiply_accumulate_long(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_signed_multiply_word(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_signed_multiply(&mut self, _: u32) {
        todo!()
    }

    pub(in crate::arm) fn arm_breakpoint(&mut self, _: u32) {
        todo!()
    }
}
