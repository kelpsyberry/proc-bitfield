#![doc = include_str!("../docs.md")]
#![no_std]
#![cfg_attr(feature = "nightly", feature(const_trait_impl))]
// Rustdoc won't actually throw an error if this is missing, but it's technically needed to compile
// the #[cfg(doc)]-guarded code
#![cfg_attr(all(doc, feature = "nightly"), feature(const_mut_refs))]
#![warn(clippy::all)]

#[doc(hidden)]
pub mod __private {
    pub use static_assertions;
}

/// The main focus of the crate.
pub use macros::bitfield;

mod conv;
pub use conv::*;
mod traits;
pub use traits::*;

#[cfg(doc)]
extern crate self as proc_bitfield;

#[cfg(doc)]
mod example;
#[cfg(doc)]
pub use example::*;
