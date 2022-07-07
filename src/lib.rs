#![doc = include_str!("../README.md")]
#![no_std]
#![cfg_attr(feature = "nightly", feature(const_trait_impl))]
#![cfg_attr(all(doc, feature = "nightly"), feature(const_mut_refs))]
#![warn(clippy::all)]

mod conv;
pub use conv::*;

pub use macros::*;
pub use static_assertions;

/// A trait to read or modify a range of bits inside a value.
pub trait BitRange<T> {
    /// Read the given bit range inside `self` as a value of type `T`
    fn bit_range<const START: usize, const END: usize>(self) -> T;

    #[must_use]
    /// Set the given bit range inside `self` to a value of type `T`.
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
                const VALUE_BIT_LEN: usize = core::mem::size_of::<$value>() << 3;
                let selected = END - START;
                let mask = (if selected == VALUE_BIT_LEN {
                    <$value>::MAX
                } else {
                    ((1 as $value) << selected) - 1
                } as $storage)
                    << START;
                (self & !mask) | ((value as $storage) << START & mask)
            }
        }
    };
}

macro_rules! impl_bitrange_for_types {
    (=> $($dst_ty: ty),*) => {};
    ($src_ty: ty $(, $other_src_ty: ty)* => $($dst_ty: ty),*) => {
        $(
            #[cfg(feature = "nightly")]
            impl_bitrange!($src_ty, $dst_ty, const);
            #[cfg(not(feature = "nightly"))]
            impl_bitrange!($src_ty, $dst_ty);
        )*
        impl_bitrange_for_types!($($other_src_ty),* => $($dst_ty),*);
    };
    ($($($src_ty: ty),* => $($dst_ty: ty),*);*) => {
        $(
            impl_bitrange_for_types!($($src_ty),* => $($dst_ty),*);
        )*
    };
}

impl_bitrange_for_types!(
    u8, i8 => u8, i8;
    u16, i16 => u8, u16, i8, i16;
    u32, i32 => u8, u16, u32, i8, i16, i32;
    u64, i64 => u8, u16, u32, u64, i8, i16, i32, i64;
    u128, i128 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128
);

#[cfg(target_pointer_width = "16")]
impl_bitrange_for_types!(
    usize, isize => u8, u16, i8, i16
);

#[cfg(target_pointer_width = "32")]
impl_bitrange_for_types!(
    usize, isize => u8, u16, u32, i8, i16, i32
);

#[cfg(target_pointer_width = "64")]
impl_bitrange_for_types!(
    usize, isize => u8, u16, u32, u64, i8, i16, i32, i64
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

#[cfg(feature = "nightly")]
macro_rules! impl_bit_for_types {
    ($($t: ty),*) => {
        $(impl_bit!($t, const);)*
    };
}

#[cfg(not(feature = "nightly"))]
macro_rules! impl_bit_for_types {
    ($($t: ty),*) => {
        $(impl_bit!($t);)*
    };
}

impl_bit_for_types!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

#[cfg(doc)]
extern crate self as proc_bitfield;

#[cfg(all(doc, feature = "nightly"))]
bitfield! {
    /// A sample bitfield showcasing the library's features (open the source code to view).
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub const struct Example(pub u16): Debug {
        // A single field spanning the entire bitfield, using an unbounded range
        pub raw: u16 @ ..,

        // Single-bit flags
        pub vblank: bool [read_only] @ 0,
        pub hblank: bool [write_only] @ 1,
        pub vcount_match: bool @ 2,

        // Multi-bit field, specified using an inclusive range
        pub irq_mask: u8 @ 3..=5,

        // Bit 6 is ignored

        // Single-bit field, specified using an exclusive range
        pub vcount_compare_high: u8 @ 7..8,

        // 8-bit field specified using its start bit and length
        pub vcount_compare_low: u8 @ 8; 8,
    }
}

#[cfg(all(doc, not(feature = "nightly")))]
bitfield! {
    /// A sample bitfield showcasing the library's features (open the source code to view).
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Example(pub u16): Debug {
        // A single field spanning the entire bitfield, using an unbounded range
        pub raw: u16 @ ..,

        // Single-bit flags
        pub vblank: bool [read_only] @ 0,
        pub hblank: bool [write_only] @ 1,
        pub vcount_match: bool @ 2,

        // Multi-bit field, specified using an inclusive range
        pub irq_mask: u8 @ 3..=5,

        // Bit 6 is ignored

        // Single-bit field, specified using an exclusive range
        pub vcount_compare_high: u8 @ 7..8,

        // 8-bit field specified using its start bit and length
        pub vcount_compare_low: u8 @ 8; 8,
    }
}
