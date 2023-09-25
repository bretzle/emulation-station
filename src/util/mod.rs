mod shared;
mod ringbuf;

pub use ringbuf::*;
pub use shared::*;

/// Create a C-style bitfield
///
/// ```
/// bitfield! {
///     #[derive(Default, Copy, Clone)]
///     pub struct StatusRegister(u32) {
///         pub mode: u8 [Mode] => 0 | 4,
///         pub thumb: bool => 5,
///         pub f: bool => 6,
///         pub i: bool => 7,
///         pub q: bool => 27,
///         pub v: bool => 28,
///         pub c: bool => 29,
///         pub z: bool => 30,
///         pub n: bool => 31
///     }
/// }
/// ```
#[macro_export]
macro_rules! bitfield {
    (
        $(#[derive($($m:meta),+)])?
        $vis:vis struct $struct_name:ident($ivis:vis $raw_type:ident) {
            $( $field_vis:vis $field_name:ident: $field_ty:ty $([ $real_ty:ty ])? => $bit_val:tt $(| $bit_end:tt)? ),+
        }
    ) => {
        $(#[derive($($m),+)])?
        $vis struct $struct_name($ivis $raw_type);
        #[allow(dead_code)]
        impl $struct_name {
            pub const fn new(bits: $raw_type) -> Self {
                Self(bits)
            }

            pub const fn bits(&self) -> $raw_type {
                self.0
            }

            pub fn set_bits(&mut self, bits: $raw_type) {
                self.0 = bits
            }

            $( $crate::bitfield!(@IMPL $raw_type, $field_vis $field_name: $field_ty $([$real_ty])? => $bit_val $($bit_end)?); )+
            $crate::bitfield!(@IMPL HELPER $raw_type $raw_type);
        }
    };

    (@IMPL HELPER u8 $inner:ty) => {
        pub fn byte0(&self) -> u8 {
            (self.0 & 0xFF) as u8
        }
        pub fn set_byte0(&mut self, val: u8) {
            self.0 &= !0xFF;
            self.0 |= val as $inner;
        }
    };
    (@IMPL HELPER u16 $inner:ty) => {
        $crate::bitfield!(@IMPL HELPER u8 $inner);
        pub fn byte1(&self) -> u8 {
            ((self.0 >> 8) & 0xFF) as u8
        }
        pub fn set_byte1(&mut self, val: u8) {
            self.0 &= !0xFF00;
            self.0 |= (val as $inner) << 8;
        }
    };
    (@IMPL HELPER u32 $inner:ty) => {
        $crate::bitfield!(@IMPL HELPER u16 $inner);
        pub fn byte2(&self) -> u8 {
            ((self.0 >> 16) & 0xFF) as u8
        }
        pub fn set_byte2(&mut self, val: u8) {
            self.0 &= !0xFF0000;
            self.0 |= (val as u32) << 16;
        }
        pub fn byte3(&self) -> u8 {
            ((self.0 >> 24) & 0xFF) as u8
        }
        pub fn set_byte3(&mut self, val: u8) {
            self.0 &= !0xFF000000;
            self.0 |= (val as u32) << 24;
        }
    };

    // Bitfield impls

    (@IMPL $storage:ty, $field_vis:vis $field_name:ident: $field_ty:ty => $bit_val:tt $($bit_end:tt)?) => {
        ::paste::paste! {
            #[inline]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_ty) -> Self {
                $crate::bitfield!(@MASK set $field_ty, $storage, val, self, $bit_val $(, $bit_end)?);
                self
            }

            #[inline]
            $field_vis const fn $field_name(&self) -> $field_ty {
                $crate::bitfield!(@MASK get $field_ty, self, $bit_val $(, $bit_end)?)
            }

            #[inline]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_ty) {
                $crate::bitfield!(@MASK set $field_ty, $storage, val, self, $bit_val $(, $bit_end)?)
            }
        }
    };

    (@IMPL $storage:ty, $field_vis:vis $field_name:ident: $field_ty:ty [$real_ty:ty] => $bit_val:tt $($bit_end:tt)?) => {
        ::paste::paste! {
            #[inline]
            $field_vis const fn [<with_ $field_name>](mut self, val: $real_ty) -> Self {
                $crate::bitfield!(@MASK set $field_ty, $storage, val, self, $bit_val $(, $bit_end)?);
                self
            }

            #[inline]
            $field_vis const fn $field_name(&self) -> $real_ty {
                let ret = $crate::bitfield!(@MASK get $field_ty, self, $bit_val $(, $bit_end)?);
                unsafe { ::core::mem::transmute(ret) }
            }

            #[inline]
            $field_vis fn [<set_ $field_name>](&mut self, val: $real_ty) {
                let val = val as $field_ty;
                $crate::bitfield!(@MASK set $field_ty, $storage, val, self, $bit_val $(, $bit_end)?)
            }
        }
    };

    // masks
    (@MASK get $output:ty, $self:ident, $start:tt) => {{
        ($self.0 & 1 << $start != 0) as $output
    }};

    (@MASK set $output:ty, $storage:ty, $val:ident, $self:ident, $start:tt) => {{
        $self.0 = ($self.0 & !(1 << $start)) | ($val as $storage) << $start
    }};

    (@MASK get $output:ty, $self:ident, $start:tt, $end:tt) => {{
        const VALUE_BIT_LEN: usize = ::core::mem::size_of::<$output>() << 3;
        const SELECTED: usize = ($end + 1) - $start;
        (($self.0 >> $start) as $output) << (VALUE_BIT_LEN - SELECTED) >> (VALUE_BIT_LEN - SELECTED)
    }};

    (@MASK set $output:ty, $storage:ty, $val:ident, $self:ident, $start:tt, $end:tt) => {{
        const VALUE_BIT_LEN: usize = ::core::mem::size_of::<$output>() << 3;
        let selected = ($end + 1) - $start;
        let mask = (if selected == VALUE_BIT_LEN {
            <$output>::MAX
        } else {
            ((1 as $output) << selected) - 1
        } as $storage) << $start;
        $self.0 = ($self.0 & !mask) | (($val as $storage) << $start & mask);
    }};
}
