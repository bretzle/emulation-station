use crate::arm::cpu::Cpu;
use crate::arm::interpreter::instructions::ShiftType;

impl Cpu {
    pub fn alu_mov(&mut self, op2: u32, set_flags: bool) -> u32 {
        if set_flags {
            self.set_nz(op2);
        }
        op2
    }

    pub fn alu_mvn(&mut self, op2: u32, set_flags: bool) -> u32 {
        self.alu_mov(!op2, set_flags)
    }

    pub fn alu_teq(&mut self, op1: u32, op2: u32) {
        self.set_nz(op1 ^ op2)
    }

    pub fn alu_cmp(&mut self, op1: u32, op2: u32) {
        let result = op1 - op2;
        self.set_nz(result);
        self.state.cpsr.set_c(op1 >= op2);
        self.state.cpsr.set_v(sub_overflow(op1, op2, result))
    }

    pub fn alu_cmn(&mut self, op1: u32, op2: u32) {
        let result = op1 + op2;
        self.set_nz(result);
        self.state.cpsr.set_c(result < op1);
        self.state.cpsr.set_v(add_overflow(op1, op2, result))
    }

    pub fn alu_tst(&mut self, op1: u32, op2: u32) {
        self.set_nz(op1 & op2)
    }

    pub fn alu_add(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result = op1 + op2;
        if set_flags {
            self.set_nz(result);
            self.state.cpsr.set_c(result < op1);
            self.state.cpsr.set_v(add_overflow(op1, op2, result));
        }
        result
    }

    pub fn alu_adc(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result64 = (op1 as u64) + (op2 as u64) + self.state.cpsr.c() as u64;
        let result = result64 as u32;
        if set_flags {
            self.set_nz(result);
            self.state.cpsr.set_c(result64 >> 32 != 0);
            self.state.cpsr.set_v(add_overflow(op1, op2, result));
        }
        result
    }

    pub fn alu_sbc(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let op3 = self.state.cpsr.c() as u32 ^ 1;
        let result = op1 - op2 - op3;
        if set_flags {
            self.set_nz(result);
            self.state.cpsr.set_c((op1 as u64) >= ((op2 as u64) + (op3 as u64)));
            self.state.cpsr.set_v(sub_overflow(op1, op2, result));
        }
        result
    }

    pub fn alu_eor(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result = op1 ^ op2;
        if set_flags {
            self.set_nz(result)
        }
        result
    }

    pub fn alu_sub(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result = op1 - op2;
        if set_flags {
            self.set_nz(result);
            self.state.cpsr.set_c(op1 >= op2);
            self.state.cpsr.set_v(sub_overflow(op1, op2, result));
        }
        result
    }

    pub fn alu_orr(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result = op1 | op2;
        if set_flags {
            self.set_nz(result)
        }
        result
    }

    pub fn alu_bic(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result = op1 & !op2;
        if set_flags {
            self.set_nz(result)
        }
        result
    }

    pub fn alu_and(&mut self, op1: u32, op2: u32, set_flags: bool) -> u32 {
        let result = op1 & op2;
        if set_flags {
            self.set_nz(result)
        }
        result
    }

    // todo: can this be replaced with overflowing_shl ???
    pub fn alu_lsl(&mut self, val: u32, amt: u32, carry: &mut bool) -> u32 {
        if amt == 0 {
            return val;
        }

        let mut result = 0;
        if amt >= 32 {
            *carry = if amt > 32 { false } else { val & 1 != 0 }
        } else {
            result = val << amt;
            *carry = (val >> (32 - amt)) & 1 != 0;
        }
        result
    }

    pub fn alu_lsr(&mut self, val: u32, amt: u32, carry: &mut bool, imm: bool) -> u32 {
        let result;
        if imm {
            if amt == 0 {
                result = 0;
                *carry = val >> 31 != 0;
            } else {
                result = val >> amt;
                *carry = (val >> (amt - 1)) & 1 != 0;
            }
        } else {
            if amt == 0 {
                result = val;
            } else if amt < 32 {
                result = val >> amt;
                *carry = (val >> (amt - 1)) & 1 != 0;
            } else if amt == 32 {
                result = 0;
                *carry = val >> 31 != 0;
            } else {
                result = 0;
                *carry = false;
            }
        }
        result
    }

    pub fn alu_asr(&mut self, val: u32, amt: u32, carry: &mut bool, imm: bool) -> u32 {
        let result;
        let msb = val >> 31;
        if imm {
            if amt == 0 {
                result = 0xffffffff * msb;
                *carry = msb != 0;
            } else {
                result = (val >> amt) | ((0xffffffff * msb) << (32 - amt));
                *carry = (val >> (amt - 1)) & 0x1 != 0;
            }
        } else {
            if amt == 0 {
                result = val;
            } else if amt < 32 {
                result = (val >> amt) | ((0xffffffff * msb) << (32 - amt));
                *carry = (val >> (amt - 1)) & 0x1 != 0;
            } else {
                result = 0xffffffff * msb;
                *carry = msb != 0;
            }
        }
        result
    }

    pub fn alu_ror(&mut self, val: u32, amt: u32, carry: &mut bool, imm: bool) -> u32 {
        let result;
        if imm {
            if amt == 0 {
                result = ((*carry as u32) << 31) | (val >> 1);
                *carry = val & 1 != 0;
            } else {
                result = val.rotate_right(amt);
                *carry = (val >> (amt - 1)) & 1 != 0;
            }
        } else {
            if amt == 0 {
                result = val;
            } else if (amt & 0x1f) == 0 {
                result = val;
                *carry = val >> 31 != 0;
            } else {
                result = val.rotate_right(amt & 0x1f);
                *carry = (val >> ((amt & 0x1f) - 1)) & 1 != 0;
            }
        }
        result
    }

    pub fn barrel_shifter(&self, val: u32, shift_type: ShiftType, amount: u32, imm: bool) -> (u32, Option<bool>) {
        match shift_type {
            ShiftType::LSL => arithmetic::lsl(val, amount),
            ShiftType::LSR => arithmetic::lsr(val, amount, imm),
            ShiftType::ASR => arithmetic::asr(val, amount, imm),
            ShiftType::ROR => {
                if imm && amount == 0 {
                    arithmetic::rrx(val, self.state.cpsr.c())
                } else {
                    arithmetic::ror(val, amount)
                }
            }
        }
    }
}

mod arithmetic {
    pub const fn lsl(val: u32, amount: u32) -> (u32, Option<bool>) {
        if amount == 0 {
            return (val, None);
        }

        if amount >= 32 {
            return (0, Some(amount == 32 && (val & 1 != 0)));
        }

        (val << amount, Some(val >> (32 - amount) & 1 != 0))
    }

    pub const fn lsr(val: u32, mut amount: u32, imm: bool) -> (u32, Option<bool>) {
        if amount == 0 {
            if imm {
                amount = 32;
            } else {
                return (val, None);
            }
        }

        if amount >= 32 {
            return (0, Some((amount == 32) && (val >> 31 != 0)));
        }

        (val >> amount, Some((val >> (amount - 1)) & 0x1 != 0))
    }

    pub const fn asr(val: u32, mut amount: u32, imm: bool) -> (u32, Option<bool>) {
        if amount == 0 {
            if imm {
                amount = 32;
            } else {
                return (val, None);
            }
        }

        if amount >= 32 {
            return ((val as i32 >> 31) as u32, Some(val >> 31 != 0));
        }

        ((val as i32 >> amount) as u32, Some((val >> (amount - 1)) & 1 != 0))
    }

    pub const fn rrx(val: u32, carry: bool) -> (u32, Option<bool>) {
        let msb = (carry as u32) << 31;
        let carry = val & 1 != 0;
        ((val >> 1) | msb, Some(carry))
    }

    pub const fn ror(val: u32, mut amount: u32) -> (u32, Option<bool>) {
        if amount == 0 {
            return (val, None);
        }

        amount &= 0x1f;
        let result = val.rotate_right(amount);
        (result, Some(result >> 31 != 0))
    }
}

// todo: move into utility package
pub const fn add_overflow(lhs: u32, rhs: u32, result: u32) -> bool {
    ((!(lhs ^ rhs) & (rhs ^ result)) >> 31) != 0
}

pub const fn sub_overflow(lhs: u32, rhs: u32, result: u32) -> bool {
    (((lhs ^ rhs) & (lhs ^ result)) >> 31) != 0
}
