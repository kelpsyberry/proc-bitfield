# Usage example
([Generated type docs](https://docs.rs/proc-bitfield/latest/proc_bitfield/example/struct.UnwrapBitRangeExample.html))

```rust
#[derive(UnwrapBitRange)]
pub struct U8(u8);

impl TryFrom<u16> for U8 { ... }
impl From<U8> for u16 { ... }
```

This will implement `BitRange<U8> for u16`, allowing it to be used as a field inside any
bitfield using a `u16` as its storage type, and unwrapping the result on reads.

This derive is especially useful when combined with `ConvRaw`, in which case it will allow enums
to be used as bitfield fields, unwrapping the results of trying to convert the raw value back to
an enum variant.
