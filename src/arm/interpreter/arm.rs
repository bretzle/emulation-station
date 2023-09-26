use log::{error, warn};

use crate::arm::coprocessor::Coprocessor;
use crate::arm::cpu::{Arch, Cpu};
use crate::arm::interpreter::alu::{add_overflow, sub_overflow};
use crate::arm::interpreter::instructions::*;
use crate::arm::memory::Memory;
use crate::arm::state::{Bank, Mode, GPR};

#[allow(dead_code)]
impl<M: Memory, C: Coprocessor> Cpu<M, C> {
    pub(in crate::arm) fn arm_branch_link_maybe_exchange(&mut self, instruction: u32) {
        if (instruction & 0xf0000000) != 0xf0000000 {
            self.arm_branch_link(instruction);
        } else {
            self.arm_branch_link_exchange(instruction);
        }
    }

    pub(in crate::arm) fn arm_branch_exchange(&mut self, instruction: u32) {
        let ArmBranchExchange { rm } = ArmBranchExchange::decode(instruction);
        if self.state.gpr[rm as usize] & 1 != 0 {
            self.state.cpsr.set_thumb(true);
            self.state.gpr[15] = self.state.gpr[rm as usize] & !1;
            self.thumb_flush_pipeline();
        } else {
            self.state.gpr[15] = self.state.gpr[rm as usize] & !3;
            self.arm_flush_pipeline();
        }
    }

    fn arm_branch_link(&mut self, instruction: u32) {
        let ArmBranchLink {
            link,
            offset,
            condition: _,
        } = ArmBranchLink::decode(instruction);
        if link {
            self.state.gpr[14] = self.state.gpr[15] - 4;
        }

        #[cfg(debug_assertions)]
        {
            let old = self.state.gpr[15];
            let fns: Vec<(u32, &str)> = vec![
                // (0x20002a4, "_Z11draw_stringiiPKc"),
                // (0x02000240, "_Z9draw_tileiii")
            ];
            for (addr, name) in fns {
                if self.state.gpr[15] + offset == addr {
                    log::debug!("{name}: {:x} {:08x?}", old - 8, self.state.gpr);
                }
            }
        }
        self.state.gpr[15] += offset;
        self.arm_flush_pipeline();
    }

    fn arm_branch_link_exchange(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return warn!("Interpreter: arm_branch_link_exchange executed by arm7");
        }

        let ArmBranchLinkExchange { offset } = ArmBranchLinkExchange::decode(instruction);
        self.state.gpr[14] = self.state.gpr[15] - 4;
        self.state.cpsr.set_thumb(true);
        self.state.gpr[15] += offset;
        self.thumb_flush_pipeline();
    }

    pub(in crate::arm) fn arm_count_leading_zeroes(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return self.undefined_exception();
        }

        let ArmCountLeadingZeros { rm, rd } = ArmCountLeadingZeros::decode(instruction);
        self.state.gpr[rd as usize] = self.state.gpr[rm as usize].leading_zeros();
        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_branch_link_exchange_register(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return warn!("Interpreter: arm_branch_link_exchange_register executed by arm7");
        }

        let ArmBranchExchange { rm } = ArmBranchExchange::decode(instruction);
        self.state.gpr[14] = self.state.gpr[15] - 4;
        if self.state.gpr[rm as usize] & 0x1 != 0 {
            self.state.cpsr.set_thumb(true);
            self.state.gpr[15] = self.state.gpr[rm as usize] & !0x1;
            self.thumb_flush_pipeline();
        } else {
            self.state.gpr[15] = self.state.gpr[rm as usize] & !0x3;
            self.arm_flush_pipeline();
        }
    }

    pub(in crate::arm) fn arm_single_data_swap(&mut self, instruction: u32) {
        let ArmSingleDataSwap { rm, rd, rn, byte } = ArmSingleDataSwap::decode(instruction);
        let addr = self.state.gpr[rn as usize];
        let data;

        if byte {
            data = self.memory.read_byte(addr) as u32;
            self.memory
                .write_byte(addr, self.state.gpr[rm as usize] as u8);
        } else {
            data = self.read_word_rotate(addr);
            self.memory.write_word(addr, self.state.gpr[rm as usize]);
        }

        self.state.gpr[rd as usize] = data;
        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_multiply(&mut self, instruction: u32) {
        let ArmMultiply {
            set_flags,
            accumulate,
            rm,
            rs,
            rn,
            rd,
        } = ArmMultiply::decode(instruction);
        let mut result = self.state.gpr[rm as usize] * self.state.gpr[rs as usize];

        if accumulate {
            result += self.state.gpr[rn as usize]
        }

        if set_flags {
            self.set_nz(result)
        }

        self.state.gpr[rd as usize] = result;
        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_saturating_add_subtract(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return self.undefined_exception();
        }

        let ArmSaturatingAddSubtract {
            rm,
            rd,
            rn,
            double_rhs,
            sub,
        } = ArmSaturatingAddSubtract::decode(instruction);
        let lhs = self.state.gpr[rm as usize];
        let mut rhs = self.state.gpr[rn as usize];

        if rd == GPR::PC {
            todo!("handle rd == 15")
        }

        if double_rhs {
            let mut result = rhs + rhs;
            if (rhs ^ result) >> 31 != 0 {
                self.state.cpsr.set_q(true);
                result = 0x80000000 - (result >> 31);
            }
            rhs = result;
        }

        self.state.gpr[rd as usize] = if sub {
            let mut result = lhs - rhs;
            if sub_overflow(lhs, rhs, result) {
                self.state.cpsr.set_q(true);
                result = 0x80000000 - (result >> 31);
            }
            result
        } else {
            let mut result = lhs + rhs;
            if add_overflow(lhs, rhs, result) {
                self.state.cpsr.set_q(true);
                result = 0x80000000 - (result >> 31);
            }
            result
        };

        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_multiply_long(&mut self, instruction: u32) {
        let ArmMultiplyLong {
            set_flags,
            accumulate,
            sign,
            rm,
            rs,
            rdlo,
            rdhi,
        } = ArmMultiplyLong::decode(instruction);

        const fn sign_extend(x: u32) -> i64 {
            (x as i64).wrapping_shl(32).wrapping_shr(32)
        }

        let mut result = if sign {
            sign_extend(self.state.gpr[rm as usize]) * sign_extend(self.state.gpr[rs as usize])
        } else {
            (self.state.gpr[rm as usize] as i64) * (self.state.gpr[rs as usize] as i64)
        };

        if accumulate {
            result += ((self.state.gpr[rdhi as usize] as i64) << 32)
                | (self.state.gpr[rdlo as usize] as i64)
        }

        if set_flags {
            self.state.cpsr.set_n(result >> 63 != 0);
            self.state.cpsr.set_z(result == 0);
        }

        self.state.gpr[rdhi as usize] = (result >> 32) as u32;
        self.state.gpr[rdlo as usize] = (result & 0xffffffff) as u32;
        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_halfword_data_transfer(&mut self, instruction: u32) {
        let ArmHalfwordDataTransfer {
            load,
            writeback,
            up,
            pre,
            half,
            sign,
            rd,
            rn,
            rhs,
        } = ArmHalfwordDataTransfer::decode(instruction);

        if rd == GPR::PC {
            error!("Interpreter: handle rd == 15 in arm_halfword_data_transfer")
        }

        let mut addr = self.state.gpr[rn as usize];
        let mut do_writeback = !load || rd != rn;
        let mut op2 = match rhs {
            ArmHalfwordDataTransferRhs::Imm(val) => val,
            ArmHalfwordDataTransferRhs::Reg(rm) => self.state.gpr[rm as usize],
        };

        if !up {
            op2 *= u32::MAX;
        }

        if pre {
            addr += op2;
        }

        self.state.gpr[15] += 4;

        match (half, sign) {
            (true, true) => {
                if load {
                    self.state.gpr[rd as usize] =
                        sign_extend::<16>(self.memory.read_half(addr) as _);
                } else if self.arch == Arch::ARMv5 {
                    if rd as usize & 1 != 0 {
                        error!("Interpreter: undefined strd exception")
                    }

                    self.memory.write_word(addr, self.state.gpr[rd as usize]);
                    self.memory
                        .write_word(addr + 4, self.state.gpr[rd as usize + 1]);
                }
            }
            (true, _) => {
                if load {
                    self.state.gpr[rd as usize] = self.memory.read_half(addr) as u32;
                } else {
                    self.memory
                        .write_half(addr, self.state.gpr[rd as usize] as u16);
                }
            }
            (_, true) => {
                if load {
                    self.state.gpr[rd as usize] =
                        sign_extend::<8>(self.memory.read_byte(addr) as u32);
                } else if self.arch == Arch::ARMv5 {
                    if rd as usize & 0x1 != 0 {
                        error!("Interpreter: undefined ldrd exception")
                    }

                    self.state.gpr[rd as usize] = self.memory.read_word(addr);
                    self.state.gpr[rd as usize + 1] = self.memory.read_word(addr + 4);

                    do_writeback = rn as usize != (rd as usize + 1);

                    if rd == GPR::LR {
                        self.arm_flush_pipeline()
                    }
                }
            }
            _ => {}
        }

        if do_writeback {
            if !pre {
                self.state.gpr[rn as usize] += op2;
            } else if writeback {
                self.state.gpr[rn as usize] = addr;
            }
        }
    }

    pub(in crate::arm) fn arm_status_load(&mut self, instruction: u32) {
        let ArmStatusLoad { spsr, rd } = ArmStatusLoad::decode(instruction);
        if spsr {
            self.state.gpr[rd as usize] = self.state.spsr().0;
        } else {
            self.state.gpr[rd as usize] = self.state.cpsr.0;
        }
        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_status_store_register(&mut self, instruction: u32) {
        let ArmStatusStore { spsr, mask, rhs } = ArmStatusStore::decode(instruction);
        let val = match rhs {
            ArmStatusStoreRhs::Imm(_) => unreachable!(),
            ArmStatusStoreRhs::Reg(rm) => self.state.gpr[rm as usize],
        };

        if spsr {
            let spsr = self.state.spsr_mut();
            spsr.0 = (spsr.0 & !mask) | (val & mask);
        } else {
            if mask & 0xff != 0 {
                self.switch_mode((val & 0x1f).into())
            }
            self.state.cpsr.0 = (self.state.cpsr.0 & !mask) | (val & mask);
        }

        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_status_store_immediate(&mut self, instruction: u32) {
        let ArmStatusStore { spsr, mask, rhs } = ArmStatusStore::decode(instruction);
        let val = match rhs {
            ArmStatusStoreRhs::Imm(rotated) => rotated,
            ArmStatusStoreRhs::Reg(_) => unreachable!(),
        };

        if spsr {
            let spsr = self.state.spsr_mut();
            spsr.0 = (spsr.0 & !mask) | (val & mask);
        } else {
            if mask & 0xff != 0 {
                self.switch_mode((val & 0x1f).into())
            }
            self.state.cpsr.0 = (self.state.cpsr.0 & !mask) | (val & mask)
        }

        self.state.gpr[15] += 4;
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
            for i in (0..16).rev() {
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
                continue;
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
        let ArmSingleDataTransfer {
            load,
            writeback,
            byte,
            up,
            pre,
            rd,
            rn,
            condition: _,
            rhs,
        } = ArmSingleDataTransfer::decode(instruction);
        let mut addr = self.state.gpr[rn as usize];
        let do_writeback = !load || rd != rn;

        let mut op2 = match rhs {
            ArmSingleDataTransferRhs::Imm(imm) => imm,
            ArmSingleDataTransferRhs::Reg {
                rm,
                shift_type,
                amount,
            } => {
                let (result, _carry) =
                    self.barrel_shifter(self.state.gpr[rm as usize], shift_type, amount, true);
                result
            }
        };

        if !up {
            op2 *= u32::MAX
        }

        if pre {
            addr += op2;
        }

        self.state.gpr[15] += 4;

        if load {
            if byte {
                self.state.gpr[rd as usize] = self.memory.read_byte(addr) as u32;
            } else {
                self.state.gpr[rd as usize] = self.read_word_rotate(addr);
            }
        } else {
            if byte {
                self.memory
                    .write_byte(addr, self.state.gpr[rd as usize] as u8)
            } else {
                self.memory.write_word(addr, self.state.gpr[rd as usize])
            }
        }

        if do_writeback {
            if !pre {
                self.state.gpr[rn as usize] += op2;
            } else if writeback {
                self.state.gpr[rn as usize] = addr;
            }
        }

        if load && rd == GPR::PC {
            if self.arch == Arch::ARMv5 && self.state.gpr[15] & 1 != 0 {
                self.state.cpsr.set_thumb(true);
                self.state.gpr[15] &= !1;
                self.thumb_flush_pipeline();
            } else {
                self.state.gpr[15] &= !3;
                self.arm_flush_pipeline();
            }
        }
    }

    pub(in crate::arm) fn arm_data_processing(&mut self, instruction: u32) {
        let ArmDataProcessing {
            set_flags,
            rd,
            rn,
            opcode,
            condition: _,
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
        *self.state.spsr_at(Bank::SVC) = self.state.cpsr;
        self.switch_mode(Mode::Supervisor);

        self.state.cpsr.set_i(true);
        self.state.gpr[14] = self.state.gpr[15] - 4;
        self.state.gpr[15] = self.coprocessor.get_exception_base() + 0x08;
        self.arm_flush_pipeline();
    }

    pub(in crate::arm) fn arm_signed_multiply_accumulate_long(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return;
        }

        let ArmSignedMultiplyAccumulateLong {
            rm,
            rs,
            rn,
            rd,
            x,
            y,
        } = ArmSignedMultiplyAccumulateLong::decode(instruction);
        let rdhilo = (((self.state.gpr[rd as usize] as u64) << 32)
            | (self.state.gpr[rn as usize] as u64)) as i64;

        let lhs = if x {
            (self.state.gpr[rm as usize] >> 16) as i16 as i64
        } else {
            (self.state.gpr[rm as usize]) as i16 as i64
        };

        let rhs = if y {
            (self.state.gpr[rs as usize] >> 16) as i16 as i64
        } else {
            (self.state.gpr[rs as usize]) as i16 as i64
        };

        let result = (lhs * rhs) + rdhilo;
        self.state.gpr[rn as usize] = (result & 0xffffffff) as u32;
        self.state.gpr[rd as usize] = (result >> 32) as u32;
        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_signed_multiply_word(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return;
        }

        let ArmSignedMultiplyWord {
            rm,
            rs,
            rn,
            rd,
            accumulate,
            y,
        } = ArmSignedMultiplyWord::decode(instruction);
        let result = if y {
            (((self.state.gpr[rm as usize] as i32) * ((self.state.gpr[rs as usize] >> 16) as i32))
                >> 16) as u32
        } else {
            (((self.state.gpr[rm as usize] as i32) * (self.state.gpr[rs as usize] as i16 as i32))
                >> 16) as u32
        };

        if accumulate {
            let operand = self.state.gpr[rn as usize];
            self.state.gpr[rd as usize] = result + operand;

            if add_overflow(result, operand, self.state.gpr[rd as usize]) {
                self.state.cpsr.set_q(true)
            }
        } else {
            self.state.gpr[rd as usize] = result;
        }

        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_signed_multiply(&mut self, instruction: u32) {
        if self.arch == Arch::ARMv4 {
            return;
        }

        let ArmSignedMultiply {
            rm,
            rs,
            rn,
            rd,
            accumulate,
            x,
            y,
        } = ArmSignedMultiply::decode(instruction);

        let lhs = if x {
            (self.state.gpr[rm as usize] >> 16) as i16 as u32
        } else {
            self.state.gpr[rm as usize] as i16 as u32
        };
        let rhs = if y {
            (self.state.gpr[rs as usize] >> 16) as i16 as u32
        } else {
            self.state.gpr[rs as usize] as i16 as u32
        };

        let result = lhs * rhs;

        if accumulate {
            let operand = self.state.gpr[rn as usize];
            self.state.gpr[rd as usize] = result + operand;

            if add_overflow(result, operand, self.state.gpr[rd as usize]) {
                self.state.cpsr.set_q(true);
            }
        } else {
            self.state.gpr[rd as usize] = result;
        }

        self.state.gpr[15] += 4;
    }

    pub(in crate::arm) fn arm_breakpoint(&mut self, _: u32) {
        todo!()
    }
}
