## Usage example
([Generated type docs](https://docs.rs/proc-bitfield/latest/proc_bitfield/example/struct.ConvRawExample.html))

```rust
# use proc_bitfield::ConvRaw;
/// An enum showcasing the `ConvRaw` derive.
#[derive(ConvRaw)]
pub enum ConvRawIntExample {
    A,
    B = 2,
    C,
    D = -1,
    E = 1,
    F = -128,
    G = 128,
}

/// An enum showcasing the `ConvRaw` derive when allowing for boolean values.
#[derive(ConvRaw)]
pub enum ConvRawBoolExample {
    False, // Implicitly, this value is treated as 0 (false).
    True,
}
```

This will implement:
- `TryFrom<T> for ConvRawIntExample`, `TryFrom<T> for ConvRawBoolExample` for all integer types `T`
- `UnsafeFrom<T> for ConvRawIntExample`, `UnsafeFrom<T> for ConvRawBoolExample` for all integer types `T`
- `From<ConvRawIntExample> for T`, `From<ConvRawBoolExample> for T` for all integer types `T` that contain all discriminants; in this case, all signed integer types with `>= 16` bits (`i16`, `i32`, `i64`, `i128`)
- `From<bool> for ConvRawBoolExample`
- `From<ConvRawBoolExample> for bool`
