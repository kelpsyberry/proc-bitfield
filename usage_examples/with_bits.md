## Usage example: returning a modified bitfield with `with_bits!`

As for [`bits!`](https://docs.rs/proc-bitfield/latest/proc_bitfield/macro.bits.html), the field's type `T` can be specified by prepending `T @` to the bit range, or additionally by casting the new value with `as T`. However, in this example it can be inferred.

```rust
# use proc_bitfield::with_bits;
#
let a = 0x1234_u16;

// A single field spanning the entire bitfield, using an unbounded range:
// NOTE: In this case, the bitfield's storage type needs to be specified by appending `as T`
assert_eq!(with_bits!(a as u16, .. = 0xFFFF), 0xFFFF); // Bits 0 to 31

// Multi-bit field, specified using an inclusive range:
assert_eq!(with_bits!(a, 0..=3 = 0xF), 0x123F);        // Bits 0 to 3

// Multi-bit field, specified using an exclusive range:
assert_eq!(with_bits!(a, 4..8 = 0xF), 0x12F4);         // Bits 4 to 7

// Multi-bit field specified using its start bit and length:
assert_eq!(with_bits!(a, 8; 4 = 0xF), 0x1F34);         // Bits 8 to 11

// Single-bit field, specified using an inclusive range:
assert_eq!(with_bits!(a, 12..=12 = 1), 0x1234);        // Bit 12

// Single-bit field, specified using an exclusive range:
assert_eq!(with_bits!(a, 13..14 = 1), 0x3234);         // Bit 13

// Single-bit field, specified using its start bit and a length of 1:
assert_eq!(with_bits!(a, 14; 1 = 1), 0x5234);          // Bit 14

// Single-bit boolean flag, specified using a single bit position:
assert_eq!(with_bits!(a, 15 = true), 0x9234);          // Bit 15
```
