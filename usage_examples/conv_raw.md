## Usage example
([Generated type docs](https://docs.rs/proc-bitfield/latest/proc_bitfield/example/struct.ConvRawExample.html))

```rust
# use proc_bitfield::ConvRaw;
/// An enum showcasing the `ConvRaw` derive for converting from/into integers.
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
```

This will implement:
- `TryFrom<T> for ConvRawIntExample` for all integer types `T`
- `UnsafeFrom<T> for ConvRawIntExample` for all integer types `T`
- `From<ConvRawIntExample> for T` for all integer types `T` that contain all discriminants; in this case, all signed integer types with `>= 16` bits (`i16`, `i32`, `i64`, `i128`)
- `From<bool> for ConvRawBoolExample`
- `From<ConvRawBoolExample> for bool`

```rust
# use proc_bitfield::ConvRaw;
/// An enum showcasing the `ConvRaw` derive for converting from/into booleans.
#[derive(ConvRaw)]
pub enum ConvRawBoolExample {
    False, // Implicitly, this value is treated as 0 (false).
    True,
}
```

This will implement:
- `TryFrom<T> for ConvRawBool` for all integer types `T`
- `UnsafeFrom<T> for ConvRawBool` for all integer types `T`
- `From<ConvRawBool> for T` for all integer types `T` that contain all discriminants; in this case, all integer types
- `From<bool> for ConvRawBoolExample`
- `From<ConvRawBoolExample> for bool`
