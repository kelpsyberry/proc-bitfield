## 0.5.2
- Added the ability to specify field ranges using arbitrary expressions in addition to literals, by enclosing the expressions in parens (`(<expr>)`)
- Reverted the fix from v0.5.1 due to it hindering other optimizations and the original issue only triggering in extremely specific cases

## 0.5.1
- Fixed a missed LLVM optimization for single-bit writes to bitfields on AArch64 by using inline assembly; this requires both a nightly compiler and for the `aarch64-bit-fix` feature flag to be enabled (to prevent breakage due to `unsafe` usage)

## 0.5.0
- **BREAKING**: Renamed `FromRaw`, `IntoRaw` and `DerefRaw` to `FromStorage`, `IntoStorage` and `DerefStorage`
- Added nested bitfield support, used by specifying a field's type as `nested T`
- Made bitfields `#[repr(transparent)]`
- Added the ability to pack fields next to each other with `above; bits`, `below; bits`, `above` and `below` *FieldRange* specifiers
- Fixed generic bitfield support; however, if used, compile-time checks will be converted to runtime ones due to language limitations

## 0.4.0
- **BREAKING**: Changed the default behavior of unsafe conversions to mark the accessor as unsafe; the old behavior (safe accessor that performs an unsafe conversion) can be obtained by adding a `!` suffix to the attribute name, i.e. `unsafe_get! T`
- Added field conversions through functions with `get_fn`, `set_fn`, `unsafe_get_fn`, `unsafe_set_fn`, `try_get_fn`, `try_set_fn`, `unwrap_get_fn` and `unwrap_set_fn`
- Made the `ConvRaw` derive implement `bool` conversions for enums with only two variants with discriminants 0 and 1 (in any order)
- Changes to the `BitRange<T>` and `Bit` traits:
    - Renamed and split the `BitRange<T>` trait, into `Bits<T>`, `WithBits<T>` and `SetBits<T>`
    - Split the `Bit` trait into `Bit`, `WithBit` and `SetBit`
    - `Bits<T>` and `Bit` read from a bitfield
    - `With*` traits return a changed version of the bitfield
    - `Set*` traits modify the bitfield in-place
- Changed the implementations of `UnsafeFrom<T>` and `UnsafeInto<T>` so that either `From<T>` or `Into<T>` automatically implement both
    - Specifically, `UnsafeFrom<U> for T` now gets implemented if `U: Into<T>` (implied by `T: From<U>`), and `UnsafeInto<U> for T` gets implemented if `U: UnsafeFrom<T>`
- Added built-in `*Bits` and `*Bit` implementations to use integer arrays as field and storage types, and to use unsized integer slices as storage types (only in the `*bits!` macros)

## 0.3.1
- Added `bits!`, `with_bits!` and `set_bits!` as alternatives to `bitfield!` to operate on raw "bitfield storage" values without declaring a bitfield struct
- Clarified `BitRange`'s expected behavior in the documentation

## 0.3.0
- Added `unwrap_get`/`unwrap_set`/`unwrap_both`/`unwrap` as alternatives to the `try_*` field type conversions that also unwrap the `Result`s
- Added a new derive macro named `ConvRaw` for automatic fallible enum conversions to and from integer types; this allows much easier usage of enums as bitfield fields
- Added a new derive macro named `UnwrapBitRange` to add an automatic implementation of `BitRange<T>` to any integer types that can be converted to (fallibly) and from  (infallibly) T, unwrapping on reads
- Fixed an edge case in the default `BitRange` implementation for signed storage types, and expanded the default implementations

## 0.2.4 (`proc-bitfield` only)
- Fixed `proc-bitfield-macros` dependency (0.2.3 mistakenly depended on `proc-bitfield-macros` 0.2.2)

## 0.2.3
- Made the `nightly` feature do nothing for the time being: const trait functionality has been removed from the standard library and the entire const trait system is being reworked, as described in https://github.com/rust-lang/rust/pull/110393.

## 0.2.2
- Fixed const traits on the latest nightly
- Added optional `FromRaw`, `IntoRaw` and `DerefRaw` automatic implementations
- Fixed some field type conversions failing when the required trait was not in scope
- Fixed code for the example

## 0.2.1
- Fixed the crate on the current stable Rust compiler (1.64.0), by not using the `label_break_value` feature (stabilized in 1.65.0)
- Enabled the `nightly` feature on docs.rs

## 0.2.0
- Added more examples on how to use field options
- Added `ro` and `wo`  as shorthands for `read_only` and `write_only`, respectively
- Renamed `try` to `try_both`, and added a new `try` option is a shorthand for `try_get` and `set` combined
- Added `unsafe_set` and `unsafe_both` options to perform unsafe conversions on writes (using `UnsafeInto<T>`)

## 0.1.1 (`proc-bitfield` only)
- Fixed `proc-bitfield-macros` dependency (0.1.0 mistakenly depended on `proc-bitfield-macros` 0.0.1)

## 0.1.0 (yanked)
- Fixed conversion trait docs
- Fixed interactions between write-only fields and the automatic `fmt::Debug` implementation

## 0.0.2
- Added more documentation
- Made the crate `#![no_std]`

## 0.0.1
Initial release
