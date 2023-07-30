use crate::conv::UnsafeFrom;
use core::num::TryFromIntError;
#[cfg(feature = "nightly")]
use super::UnwrapBitRangeExample;

/// Wrapper around [`NonZeroU8`](core::num::NonZeroU8) to implement conversion traits on a foreign
/// type.
#[derive(Clone, Copy, Debug)]
pub struct NonZeroU8(core::num::NonZeroU8);

impl From<NonZeroU8> for u8 {
    fn from(other: NonZeroU8) -> Self {
        other.0.into()
    }
}

impl TryFrom<u8> for NonZeroU8 {
    type Error = TryFromIntError;

    fn try_from(other: u8) -> Result<Self, Self::Error> {
        Ok(NonZeroU8(other.try_into()?))
    }
}

impl UnsafeFrom<u8> for NonZeroU8 {
    unsafe fn unsafe_from(other: u8) -> Self {
        NonZeroU8(core::num::NonZeroU8::new_unchecked(other))
    }
}

/// Wrapper around [`u16`] to implement conversion traits on a foreign type.
#[derive(Clone, Copy, Debug)]
pub struct U16(pub u16);

impl UnsafeFrom<U16> for u8 {
    unsafe fn unsafe_from(other: U16) -> Self {
        other.0 as u8
    }
}

impl TryFrom<U16> for u8 {
    type Error = TryFromIntError;

    fn try_from(other: U16) -> Result<Self, Self::Error> {
        other.0.try_into()
    }
}

/// Wrapper around [`u8`] with infallible conversions both ways.
#[derive(Clone, Copy, Debug)]
pub struct U8WithParity {
    pub raw: u8,
    pub has_even_set_bits: bool,
}

impl From<u8> for U8WithParity {
    fn from(other: u8) -> Self {
        U8WithParity {
            raw: other,
            has_even_set_bits: other.count_ones() & 1 == 0,
        }
    }
}

impl From<U8WithParity> for u8 {
    fn from(other: U8WithParity) -> Self {
        other.raw
    }
}

/// Wrapper around [`u8`] with fallible and unsafe conversion options both ways.
#[derive(Clone, Copy, Debug)]
pub struct SpuriouslyFailingU8(u8);

impl TryFrom<u8> for SpuriouslyFailingU8 {
    type Error = ();

    fn try_from(other: u8) -> Result<Self, Self::Error> {
        // Doesn't actually fail, but in real-world code this function could fail depending on
        // external factors
        Ok(SpuriouslyFailingU8(other))
    }
}

impl UnsafeFrom<u8> for SpuriouslyFailingU8 {
    unsafe fn unsafe_from(other: u8) -> Self {
        SpuriouslyFailingU8(other)
    }
}

impl TryFrom<SpuriouslyFailingU8> for u8 {
    type Error = ();

    fn try_from(other: SpuriouslyFailingU8) -> Result<Self, Self::Error> {
        // Same as above
        Ok(other.0)
    }
}

impl UnsafeFrom<SpuriouslyFailingU8> for u8 {
    unsafe fn unsafe_from(other: SpuriouslyFailingU8) -> Self {
        other.0
    }
}

#[cfg(feature = "nightly")]
impl TryFrom<u8> for UnwrapBitRangeExample {
    type Error = ();

    fn try_from(other: u8) -> Result<Self, Self::Error> {
        match NonZeroU8::try_from(other) {
            Ok(v) => Ok(UnwrapBitRangeExample(v)),
            Err(_) => Err(()),
        }
    }
}

#[cfg(feature = "nightly")]
impl From<UnwrapBitRangeExample> for u8 {
    fn from(other: UnwrapBitRangeExample) -> Self {
        other.0.into()
    }
}
