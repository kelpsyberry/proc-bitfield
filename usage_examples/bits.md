## Usage example: reading bits from a bitfield with `bits!`

In these examples, the field's type `T` needs to be specified by prepending `T @`; in cases where type inference can already detect it, that can be omitted specifying only the bit range to be accessed.

```rust
# use proc_bitfield::bits;
#
let a = 0x1234_u16;

// A single field spanning the entire bitfield, using an unbounded range:
// NOTE: In this case, the bitfield's storage type needs to be specified by appending `as T`
assert_eq!(bits!(a as u16, u16 @ ..), a); // Bits 0 to 31

// Multi-bit field, specified using an inclusive range:
assert_eq!(bits!(a, u8 @ 0..=3), 4_u8);   // Bits 0 to 3

// Multi-bit field, specified using an exclusive range:
assert_eq!(bits!(a, u8 @ 4..8), 3_u8);    // Bits 4 to 7

// Multi-bit field specified using its start bit and length:
assert_eq!(bits!(a, u8 @ 8; 4), 2_u8);    // Bits 8 to 11

// Single-bit field, specified using an inclusive range:
assert_eq!(bits!(a, u8 @ 12..=12), 1_u8); // Bit 12

// Single-bit field, specified using an exclusive range:
assert_eq!(bits!(a, u8 @ 13..14), 0_u8);  // Bit 13

// Single-bit field, specified using its start bit and a length of 1:
assert_eq!(bits!(a, u8 @ 14; 1), 0_u8);   // Bit 14

// Single-bit boolean flag, specified using a single bit position:
assert_eq!(bits!(a, 15), false);          // Bit 15
```
