## Usage example: modifying a bitfield with `set_bits!`

As for [`bits!`](https://docs.rs/proc-bitfield/latest/proc_bitfield/macro.bits.html), the field's type `T` can be specified by prepending `T @` to the bit range, or additionally by casting the new value with `as T`. However, in this example it can be inferred.

```rust
# use proc_bitfield::set_bits;
#
let mut a = 0x1234_u16;

// A single field spanning the entire bitfield, using an unbounded range:
// NOTE: In this case, the bitfield's storage type needs to be specified by appending `as T`
set_bits!(a as u16, .. = 0xFFFF); // Bits 0 to 31
assert_eq!(a, 0xFFFF);

a = 0x1234_u16;

// Multi-bit field, specified using an inclusive range:
set_bits!(a, 0..=3 = 0xF);        // Bits 0 to 3
assert_eq!(a, 0x123F);

// Multi-bit field, specified using an exclusive range:
set_bits!(a, 4..8 = 0xF);         // Bits 4 to 7
assert_eq!(a, 0x12FF);

// Multi-bit field specified using its start bit and length:
set_bits!(a, 8; 4 = 0xF);         // Bits 8 to 11
assert_eq!(a, 0x1FFF);

// Single-bit field, specified using an inclusive range:
set_bits!(a, 12..=12 = 1);        // Bit 12
assert_eq!(a, 0x1FFF);

// Single-bit field, specified using an exclusive range:
set_bits!(a, 13..14 = 1);         // Bit 13
assert_eq!(a, 0x3FFF);

// Single-bit field, specified using its start bit and a length of 1:
set_bits!(a, 14; 1 = 1);          // Bit 14
assert_eq!(a, 0x7FFF);

// Single-bit boolean flag, specified using a single bit position:
set_bits!(a, 15 = true);          // Bit 15
assert_eq!(a, 0xFFFF);
```
