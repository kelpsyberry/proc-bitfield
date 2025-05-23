use super::{Bit, Bits, SetBit, SetBits, WithBit, WithBits};

macro_rules! impl_bits_for_int_type {
    ($storage: ty, $value: ty $(, $const:tt)?) => {
        impl $($const)* Bits<$value> for $storage {
            #[inline]
            fn bits<const START: usize, const END: usize>(&self) -> $value {
                if START >= END {
                    return 0;
                }
                const VALUE_BITS: usize = <$value>::BITS as usize;
                let read_bits = END - START;
                ((*self >> START) as $value) << (VALUE_BITS - read_bits) >> (VALUE_BITS - read_bits)
            }
        }

        impl $($const)* WithBits<$value> for $storage {
            #[inline]
            fn with_bits<const START: usize, const END: usize>(self, value: $value) -> Self {
                if START >= END {
                    return self;
                }
                let written_bits = END - START;
                let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1) << START;
                (self & !mask) | ((value as $storage) << START & mask)
            }
        }

        impl $($const)* SetBits<$value> for $storage {
            #[inline]
            fn set_bits<const START: usize, const END: usize>(&mut self, value: $value) {
                if START >= END {
                    return;
                }
                *self = self.with_bits::<START, END>(value);
            }
        }
    };
}

macro_rules! impl_bits_for_int_types {
    (const => $($dst_ty: ty),*) => {};
    (=> $($dst_ty: ty),*) => {};
    (const $src_ty: ty $(, $other_src_ty: ty)* => $($dst_ty: ty),*) => {
        $(
            impl_bits_for_int_type!($src_ty, $dst_ty, const);
        )*
        impl_bits_for_int_types!(const $($other_src_ty),* => $($dst_ty),*);
    };
    ($src_ty: ty $(, $other_src_ty: ty)* => $($dst_ty: ty),* $(; $other:tt)*) => {
        $(
            impl_bits_for_int_type!($src_ty, $dst_ty);
        )*
        impl_bits_for_int_types!($($other_src_ty),* => $($dst_ty),*);
    };
}

#[cfg(not(feature = "nightly"))]
impl_bits_for_int_types!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
        => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize; const
);
#[cfg(feature = "nightly")]
impl_bits_for_int_types!(
    const u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
        => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

macro_rules! impl_bit_for_int_type {
    ($t: ty $(, $const:tt)?) => {
        impl $($const)* Bit for $t {
            #[inline]
            fn bit<const BIT: usize>(&self) -> bool {
                *self & 1 << BIT != 0
            }
        }

        impl $($const)* WithBit for $t {
            #[inline]
            fn with_bit<const BIT: usize>(self, value: bool) -> Self {
                (self & !(1 << BIT)) | (value as $t) << BIT
            }
        }

        impl $($const)* SetBit for $t {
            #[inline]
            fn set_bit<const BIT: usize>(&mut self, value: bool) {
                *self = self.with_bit::<BIT>(value);
            }
        }
    };
}

macro_rules! impl_bit_for_int_types {
    (const $($t: ty),*) => {
        $(impl_bit_for_int_type!($t, const);)*
    };
    ($($t: ty),*) => {
        $(impl_bit_for_int_type!($t);)*
    };
}

#[cfg(not(feature = "nightly"))]
impl_bit_for_int_types!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
#[cfg(feature = "nightly")]
impl_bit_for_int_types!(const u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
