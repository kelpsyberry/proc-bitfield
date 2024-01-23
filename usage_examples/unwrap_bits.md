## Usage example
([Generated type docs](https://docs.rs/proc-bitfield/latest/proc_bitfield/example/struct.UnwrapBitsExample.html))

```rust
#![feature(trivial_bounds)]
# use proc_bitfield::UnwrapBits;
# use core::num::NonZeroU8;
#[derive(UnwrapBits)]
pub struct UnwrapBitsExample(NonZeroU8);

impl TryFrom<u8> for UnwrapBitsExample {
    /* ... */
#   type Error = ();
#   fn try_from(other: u8) -> Result<Self, Self::Error> {
#       todo!();
#   }
}

impl From<UnwrapBitsExample> for u8 {
    /* ... */
#   fn from(other: UnwrapBitsExample) -> Self {
#       todo!();
#   }
}
```

This will implement `Bits<U8> for u16`, `WithBits<U8> for u16` and `SetBits<U8> for u16`, allowing it to be used as a field inside any bitfield using a `u16` as its storage type, and unwrapping the result on reads.

This derive is especially useful when combined with `ConvRaw`, in which case it will allow enums to be used as bitfield fields, unwrapping the results of trying to convert the raw value back to an enum variant.
