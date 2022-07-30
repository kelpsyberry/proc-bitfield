# `proc-bitfield`

[![crates.io](https://img.shields.io/crates/v/proc-bitfield.svg?logo=rust)](https://crates.io/crates/proc-bitfield)
[![docs.rs](https://img.shields.io/docsrs/proc-bitfield/latest.svg?logo=docs.rs)](https://docs.rs/proc-bitfield)

A Rust crate to expressively declare bitfield-like `struct`s, automatically ensuring their correctness at compile time and declaring accessors.

[API docs](https://docs.rs/proc-bitfield)

## Usage example

```rust
bitfield! {
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Example(pub u16): Debug {
        // A single field spanning the entire bitfield, using an unbounded range
        pub raw: u16 @ ..,

        // Single-bit flags
        pub vblank: bool [read_only] @ 0,
        pub hblank: bool [write_only] @ 1,
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

## License

This project is licensed under a dual MIT/Apache 2.0 license.
