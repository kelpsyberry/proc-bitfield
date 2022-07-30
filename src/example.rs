use crate::bitfield;

#[cfg(feature = "nightly")]
bitfield! {
    /// A sample bitfield showcasing the library's features (open the source code to view).
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub const struct Example(pub u16): Debug {
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

#[cfg(not(feature = "nightly"))]
bitfield! {
    /// A sample bitfield showcasing the library's features (open the source code to view).
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
