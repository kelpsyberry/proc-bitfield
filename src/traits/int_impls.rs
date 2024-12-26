use super::{Bit, Bits, SetBit, SetBits, WithBit, WithBits};

macro_rules! impl_bits_for_int_type {
    ($storage: ty, $value: ty) => {
        impl Bits<$value> for $storage {
            #[inline]
            fn bits<const START: usize, const END: usize>(&self) -> $value {
                const VALUE_BITS: usize = <$value>::BITS as usize;
                let read_bits = END - START;
                ((*self >> START) as $value) << (VALUE_BITS - read_bits) >> (VALUE_BITS - read_bits)
            }
        }

        impl WithBits<$value> for $storage {
            #[inline]
            fn with_bits<const START: usize, const END: usize>(self, value: $value) -> Self {
                let written_bits = END - START;
                let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1) << START;
                (self & !mask) | ((value as $storage) << START & mask)
            }
        }

        impl SetBits<$value> for $storage {
            #[inline]
            fn set_bits<const START: usize, const END: usize>(&mut self, value: $value) {
                *self = self.with_bits::<START, END>(value);
            }
        }
    };
}

macro_rules! impl_bits_for_int_types {
    (=> $($dst_ty: ty),*) => {};
    ($src_ty: ty $(, $other_src_ty: ty)* => $($dst_ty: ty),*) => {
        $(
            impl_bits_for_int_type!($src_ty, $dst_ty);
        )*
        impl_bits_for_int_types!($($other_src_ty),* => $($dst_ty),*);
    };
}

impl_bits_for_int_types!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
        => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

macro_rules! impl_bit_for_int_type {
    ($t: ty $(; $aarch64_asm: literal)?) => {
        impl Bit for $t {
            #[inline]
            fn bit<const BIT: usize>(&self) -> bool {
                *self & 1 << BIT != 0
            }
        }

        impl WithBit for $t {
            #[inline]
            #[allow(unused_mut)]
            fn with_bit<const BIT: usize>(mut self, value: bool) -> Self {
                $(
                    #[cfg(all(feature = "aarch64-bit-fix", target_arch = "aarch64"))]
                    {
                        use core::intrinsics::is_val_statically_known;
                        let prev_value = self & 1 << BIT != 0;
                        let is_same = value == prev_value;
                        let is_opposite = value == !prev_value;
                        if !(
                            is_val_statically_known(value)
                            || (is_val_statically_known(is_same) && is_same)
                            || (is_val_statically_known(is_opposite) && is_opposite)
                        ) {
                            unsafe {
                                core::arch::asm!(
                                    $aarch64_asm,
                                    self = inlateout(reg) self,
                                    value = in(reg) value as u8,
                                    BIT = const BIT,
                                    options(pure, nomem, nostack, preserves_flags)
                                );
                            }
                            return self;
                        }
                    }
                )*
                (self & !(1 << BIT)) | (value as $t) << BIT
            }
        }

        impl SetBit for $t {
            #[inline]
            fn set_bit<const BIT: usize>(&mut self, value: bool) {
                *self = self.with_bit::<BIT>(value);
            }
        }
    };
}

macro_rules! impl_bit_for_int_types {
    ($(($($t: ty),* $(; $aarch64_asm: literal)?)),*) => {
        $(
            impl_bit_for_int_types!($($t),* $(; $aarch64_asm)*);
        )*
    };
    ($($t: ty),*; $aarch64_asm: literal) => {
        $(impl_bit_for_int_type!($t; $aarch64_asm);)*
    };
    ($($t: ty),*) => {
        $(impl_bit_for_int_type!($t);)*
    };
}

impl_bit_for_int_types!(
    (u8, u16, u32, i8, i16, i32; "bfi {self:w}, {value:w}, {BIT}, #1"),
    (u64, i64, usize, isize; "bfi {self:x}, {value:x}, {BIT}, #1"),
    (u128, i128)
);
