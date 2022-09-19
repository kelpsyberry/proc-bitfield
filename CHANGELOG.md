## 0.2.1 (`proc-bitfield-macros` only)
- Fixed the crate on the current stable Rust compiler (1.64.0), by not using the `label_break_value` feature (stabilized in 1.65.0)

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
