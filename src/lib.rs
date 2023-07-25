#![doc = include_str!("../docs.md")]
#![no_std]
#![cfg_attr(all(doc, feature = "nightly"), feature(trivial_bounds))]
#![warn(clippy::all)]

#[doc(hidden)]
pub mod __private {
    pub use static_assertions;
}

/// The main focus of the crate.
pub use macros::bitfield;

/// TODO
pub use macros::ConvRaw;

#[cfg(feature = "nightly")]
/// TODO
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
