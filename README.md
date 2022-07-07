# `proc-bitfield`

A Rust crate to expressively declare bitfield-like `struct`s, automatically ensuring their correctness at compile time and declaring accessors.

## Typical usage

```rust
bitfield! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Example(pub u16): Debug {
        // A single field spanning the entire bitfield, using an unbounded range
        pub raw: u16 @ ..,

        // Single-bit flags
        pub vblank: bool @ 0,
        pub hblank: bool @ 1,
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
```

## Automatic `Debug` implementation

A `fmt::Debug` implementation can be implemented automatically for a given bitfield struct by adding `: Debug` after the tuple struct-like storage type declaration; the generated `fmt` function will output the type's raw value as well as all of its fields' values.

## `nightly` feature

Optionally, the `nightly` feature can be enabled to use `const Trait` functionality: this makes the `BitRange` and `Bit` traits be implemented using `const fn`s for all integer types, and enables the option to use `const fn`s for field accessors.

With the feature enabled, `const fn` accessors can be enabled globally for a struct by replacing `struct` with `const struct` (i.e. `const struct Example(pub u16)`), or on a field-by-field basis by prepending `const` to its type (i.e. `raw: const u16 @ ..`).
