#[derive(Default)]
pub struct MathUnit {
    divcnt: u16,
    div_numer: u64,
    div_denom: u64,
    divrem_result: u64,
    div_result: u64,
    sqrtcnt: u16,
    sqrt_param: u64,
    sqrt_result: u32,
}

impl MathUnit {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn read_divcnt(&self) -> u16 {
        self.divcnt
    }
    pub fn read_div_numer(&self) -> u64 {
        self.div_numer
    }
    pub fn read_div_denom(&self) -> u64 {
        self.div_denom
    }
    pub fn read_divrem_result(&self) -> u64 {
        self.divrem_result
    }
    pub fn read_div_result(&self) -> u64 {
        self.div_result
    }
    pub fn read_sqrtcnt(&self) -> u16 {
        self.sqrtcnt
    }
    pub fn read_sqrt_param(&self) -> u64 {
        self.sqrt_param
    }
    pub fn read_sqrt_result(&self) -> u32 {
        self.sqrt_result
    }

    pub fn write_divcnt(&mut self, val: u16, mask: u16) {
        self.divcnt = (self.divcnt & !mask) | (val & mask);
        self.start_division();
    }
    pub fn write_div_numer(&mut self, val: u64, mask: u64) {
        self.div_numer = (self.div_numer & !mask) | (val & mask);
        self.start_division();
    }
    pub fn write_div_denom(&mut self, val: u64, mask: u64) {
        self.div_denom = (self.div_denom & !mask) | (val & mask);
        self.start_division();
    }
    pub fn write_sqrtcnt(&mut self, val: u16, mask: u16) {
        self.sqrtcnt = (self.sqrtcnt & !mask) | (val & mask);
        self.start_square_root();
    }
    pub fn write_sqrt_param(&mut self, val: u64, mask: u64) {
        self.sqrt_param = (self.sqrt_param & !mask) | (val & mask);
        self.start_square_root();
    }

    fn start_division(&mut self) {
        // set the division by 0 error bit only if the full 64 bits of div_denom is 0 (even in 32 bit mode)
        if self.div_denom == 0 {
            self.divcnt |= 1 << 14;
        } else {
            self.divcnt &= !(1 << 14);
        }

        let (numer, denom) = match self.divcnt & 0x3 {
            0 => (self.div_numer as u32 as i32 as i64, self.div_denom as u32 as i32 as i64),
            1 => (self.div_numer as i64, self.div_denom as u32 as i32 as i64),
            2 => (self.div_numer as i64, self.div_denom as i64),
            _ => unreachable!(),
        };

        let special_invert = |num: &mut u64| *num ^= 0xFFFF_FFFF_0000_0000;
        if numer == i64::MIN && denom == -1 {
            self.div_result = numer as u64;
            self.divrem_result = 0;
            if self.divcnt & 0x3 == 0 {
                special_invert(&mut self.div_result)
            }
        } else if denom == 0 {
            if numer == 0 {
                self.div_result = -1i64 as u64;
            } else {
                self.div_result = (-numer.signum()) as u64;
            }
            self.divrem_result = numer as u64;
            if self.divcnt & 0x3 == 0 {
                special_invert(&mut self.div_result)
            }
        } else {
            self.div_result = (numer / denom) as u64;
            self.divrem_result = (numer % denom) as u64;
        }
    }

    fn start_square_root(&mut self) {
        // todo: can this be replaced with i64::sqrt()?
        let mut res: u32 = 0;
        let mut rem: u64 = 0;
        let mut prod: u32 = 0;

        let (mut val, nbits, topshift) = if self.sqrtcnt & 0x1 != 0 {
            (self.sqrt_param, 32, 62)
        } else {
            (self.sqrt_param & 0xFFFFFFFF, 16, 30)
        };

        for _ in 0..nbits {
            rem = (rem << 2) + ((val >> topshift) & 0x3);
            val <<= 2;
            res <<= 1;
            prod = (res << 1) + 1;

            if rem >= prod as u64 {
                rem -= prod as u64;
                res += 1;
            }
        }

        self.sqrt_result = res;
    }
}
