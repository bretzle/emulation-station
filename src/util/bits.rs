#[inline(always)]
pub const fn bit<const N: usize>(x: u32) -> bool {
    ((x >> N) & 1) != 0
}

#[inline(always)]
pub const fn get_field<const START: usize, const SIZE: usize>(val: u32) -> u32 {
    (val >> START) & !(u32::MAX << SIZE)
}

#[inline(always)]
pub const fn sign_extend<const N: usize>(val: u32) -> u32 {
    let shift = (32 - N) as u32;
    (val as i32).wrapping_shl(shift).wrapping_shr(shift) as u32
}

pub fn get_access_size(mut mask: u32) -> u32 {
    let mut size = 0;
    for _ in 0..4 {
        if mask & 0xff != 0 {
            size += 8;
        }
        mask >>= 8;
    }
    size
}

pub fn get_access_offset(mut mask: u32) -> u32 {
    let mut offset = 0;
    for _ in 0..4 {
        if mask & 0xff != 0 {
            break;
        }
        offset += 1;
        mask >>= 8;
    }
    offset
}
