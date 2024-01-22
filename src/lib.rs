#![doc = include_str!("../docs.md")]
#![no_std]
#![cfg_attr(all(doc, feature = "nightly"), feature(doc_cfg, trivial_bounds))]
#![warn(clippy::all)]

#[doc(hidden)]
pub mod __private {
    pub use static_assertions;
}

/// The main focus of the crate. Defines a bitfield struct.
#[doc = include_str!("../usage_examples/bitfield.md")]
pub use macros::bitfield;

/// Reads a single field from an anonymous bitfield, without creating a bitfield struct.
#[doc = include_str!("../usage_examples/bits.md")]
pub use macros::bits;

/// Returns an anonymous bitfield with a single field modified, without creating a bitfield struct.
#[doc = include_str!("../usage_examples/with_bits.md")]
pub use macros::with_bits;

/// Modifies a single field in an anonymous bitfield, without creating a bitfield struct.
#[doc = include_str!("../usage_examples/set_bits.md")]
pub use macros::set_bits;

/// A derive macro to implement any applicable conversion traits between an enum and the builtin
/// integer types corresponding to variant discriminants.
#[doc = include_str!("../usage_examples/conv_raw.md")]
pub use macros::ConvRaw;

#[cfg(feature = "nightly")]
#[cfg_attr(all(doc, feature = "nightly"), doc(cfg(feature = "nightly")))]
/// A derive macro to implement `BitRange<T> for U` for a type `T` and all integer bitfield storage
/// types `U`, by unwrapping the conversion results.
#[doc = include_str!("../usage_examples/unwrap_bitrange.md")]
pub use macros::UnwrapBitRange;

mod conv;
pub use conv::*;
mod traits;
pub use traits::*;

#[cfg(doc)]
extern crate self as proc_bitfield;

#[cfg(doc)]
/// Sample bitfields to showcase the crate's features
pub mod example;
