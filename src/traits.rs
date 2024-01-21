/// A trait to read or modify a range of bits inside a value.
pub trait BitRange<T> {
    /// Read the `START..END` bit range (with `END` excluded) inside `self` as a value of type `T`.
    fn bit_range<const START: usize, const END: usize>(self) -> T;

    #[must_use]
    /// Set the `START..END` bit range (with `END` excluded) inside `self` to a value of type `T`.
    fn set_bit_range<const START: usize, const END: usize>(self, value: T) -> Self;
}

/// A trait to read or modify a single bit inside a value.
pub trait Bit {
    /// Read the value of the given bit inside `self`.
    fn bit<const BIT: usize>(self) -> bool;

    /// Set the value of the given bit inside `self` to 1 if `value` is `true`, and 0 otherwise.
    #[must_use]
    fn set_bit<const BIT: usize>(self, value: bool) -> Self;
}

macro_rules! impl_bitrange {
    ($storage: ty, $value: ty$(, $const: ident)?) => {
        impl $($const)* BitRange<$value> for $storage {
            #[inline]
            fn bit_range<const START: usize, const END: usize>(self) -> $value {
                const VALUE_BIT_LEN: usize = core::mem::size_of::<$value>() << 3;
                let selected = END - START;
                ((self >> START) as $value) << (VALUE_BIT_LEN - selected)
                    >> (VALUE_BIT_LEN - selected)
            }

            #[inline]
            fn set_bit_range<const START: usize, const END: usize>(self, value: $value) -> Self {
                let selected = END - START;
                let mask = ((1 as $storage) << (selected - 1) << 1).wrapping_sub(1) << START;
                (self & !mask) | ((value as $storage) << START & mask)
            }
        }
    };
}

macro_rules! impl_bitrange_for_types {
    (=> $($dst_ty: ty),*) => {};
    ($src_ty: ty $(, $other_src_ty: ty)* => $($dst_ty: ty),*) => {
        $(
            impl_bitrange!($src_ty, $dst_ty);
        )*
        impl_bitrange_for_types!($($other_src_ty),* => $($dst_ty),*);
    };
}

impl_bitrange_for_types!(
    u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
        => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

macro_rules! impl_bit {
    ($t: ty$(, $const: ident)?) => {
        impl $($const)* Bit for $t {
            #[inline(always)]
            fn bit<const BIT: usize>(self) -> bool {
                self & 1 << BIT != 0
            }

            #[inline(always)]
            #[must_use]
            fn set_bit<const BIT: usize>(self, value: bool) -> Self {
                (self & !(1 << BIT)) | (value as $t) << BIT
            }
        }
    };
}

macro_rules! impl_bit_for_types {
    ($($t: ty),*) => {
        $(impl_bit!($t);)*
    };
}

impl_bit_for_types!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
