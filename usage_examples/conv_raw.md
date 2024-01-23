## Usage example
([Generated type docs](https://docs.rs/proc-bitfield/latest/proc_bitfield/example/struct.ConvRawExample.html))

```rust
# use proc_bitfield::ConvRaw;
/// An enum showcasing the `ConvRaw` derive.
#[derive(ConvRaw)]
pub enum ConvRawExample {
    A,
    B = 2,
    C,
    D = -1,
    E = 1,
    F = -128,
    G = 128,
}
```

This will implement:
- `TryFrom<T> for ConvRawExample` for all integer types `T`
- `UnsafeFrom<T> for ConvRawExample` for all integer types `T`
- `From<ConvRawExample> for T` for all integer types `T` that contain all discriminants; in this case, all signed integer types with `>= 16` bits (`i16`, `i32`, `i64`, `i128`)
