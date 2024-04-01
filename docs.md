# `proc-bitfield`

A Rust crate to expressively declare bitfield-like `struct`s, automatically ensuring their correctness at compile time and declaring accessors.

## `nightly` feature

Optionally, the `nightly` feature can be enabled to use experimental features exclusive to nightly Rust. This currently enables the `UnwrapBits` derive.

# The `bitfield!` macro

## Automatic trait implementations

After the struct's name and its storage type declaration, a list of automatic trait implementations can be optionally added. For example, the following declaration will result in all automatic implementations being applied:
```rust
# use proc_bitfield::bitfield;
bitfield! {
    pub struct Example(pub u8): Debug, FromRaw, IntoRaw, DerefRaw { /* ... */ }
}
```
Currently, the allowed automatic implementations are `Debug`, `FromRaw`, `IntoRaw` and `DerefRaw`.

### `Debug`

If specified, `core::fmt::Debug` will be implemented automatically for the current bitfield struct; the generated `fmt` function will output the type's raw value as well as all of its *readable* fields' values.

### `FromRaw`

If specified, `core::convert::From<$storage_ty>` will be implemented automatically for the current bitfield struct; the generated `from` function will construct an instance of the bitfield struct from the provided value directly, with no additional checks, analogously to `$bitfield_ty(raw)` in a context where the bitfield struct's raw value field is accessible. *This does not check or change the previously declared visibility of the bitfield struct's raw value field (`bitfield.0`), or any other such manually-declared fields, so care must be taken to maintain consistency.*

### `IntoRaw`

If specified, `core::convert::From<$bitfield_ty>` will be implemented automatically for the current bitfield struct's storage type (and consequently, `core::convert::Into<$storage_ty>` for the bitfield type); the generated `from` function will read the bitfield's raw value, with no additional changes, analogously to `bitfield.0` in a context where the bitfield struct's raw value field is accessible. *Analogously to `FromRaw`, care must be taken to maintain consistency with the visibility of the bitfield struct's raw value outside this implementation.*

### `DerefRaw`

If specified, `core::ops::Deref` will be implemented automatically for the current bitfield struct; the generated `deref` function will read the bitfield's raw value directly, analogously to `&bitfield.0` in a context where the bitfield struct's raw value field is accessible. *Analogously to `FromRaw`, care must be taken to maintain consistency with the visibility of the bitfield struct's raw value outside this implementation.*

## Field declarations

Fields can be declared by using the form:
> [*Visibility*] [IDENTIFIER] `:` [*Type*] (`[`(*Option* `,`)<sup>*</sup> *Option*`]`)<sup>?</sup> `@` *FieldRange*

where *FieldRange* corresponds to any of (where *L* is an alias for [*LiteralExpression*]):
- `..`, to use every bit
- *L*`..=`*L*, to use the bits specified by an inclusive range
- *L*`..`*L*, to use the bits specified by an exclusive range
- *L*`;` *L*, to use bits specified by a (start, length) pair
- *L*, to use a single bit; unlike all other specifications, this is only valid for `bool` fields, and will use the `Bit` traits instead of `Bits<T>`

*Option*s can be optionally specified in brackets, matching any of the ones defined below.

### Access restrictions

Fields are both readable and writable by default, but can be declared read-only or write-only using respectively the `read_only`/`ro` and `write_only`/`wo` options.

### Field type conversions

Fields' "raw" types as specified after the colon are restricted by `Bits<T>`, `WithBits<T>` and `SetBits<T>` (or `Bit`, `WithBit` and `SetBit` for boolean fields) implementations on the bitfield's contained type; however, accessors can perform conversions specified through optional options. These can be:
- Infallible conversions, using the `From<T>` and `Into<T>` traits, the relevant options being:
    - `get` [*Type*], specifying the type that the raw value will be converted into on reads, using `From<T>`
    - `set` [*Type*], specifying the type that will be converted into the raw value on writes, using `Into<T>`
    - [*Type*], as a shorthand for `get` [*Type*] and `set` [*Type*]
- Fallible conversions, using the `TryFrom<T>` and `TryInto<T>` traits, the relevant options being:
    - `try_get` [*Type*], specifying the type that the raw value will be fallibly converted into on reads, using `TryFrom<T>`
    - `try_set` [*Type*], specifying the type that will be fallibly converted into the raw value on writes, using `TryInto<T>`
    - `try_both` [*Type*], as a shorthand for `try_get` [*Type*] and `try_set` [*Type*]
    - `try` [*Type*], as a shorthand for `try_get` [*Type*] and `set` [*Type*]
- Unwrapping conversions, using the `TryFrom<T>` and `TryInto<T>` traits and unwrapping the conversion results, the relevant options being:
    - `unwrap_get` [*Type*], specifying the type that the raw value will be fallibly converted into and unwrapped on reads, using `TryFrom<T>` and then `Result::unwrap`
    - `unwrap_set` [*Type*], specifying the type that will be fallibly converted into the raw value and unwrapped on writes, using `TryInto<T>` and then `Result::unwrap`
    - `unwrap_both` [*Type*], as a shorthand for `unwrap_get` [*Type*] and `unwrap_set` [*Type*]
    - `try` [*Type*], as a shorthand for `unwrap_get` [*Type*] and `set` [*Type*]
- Unsafe conversions, using the `UnsafeFrom<T>` and `UnsafeInto<T>` traits, the relevant options being:
    - `unsafe_get` [*Type*], specifying the type that the raw value will be unsafely converted into on reads, using `UnsafeFrom<T>`
    - `unsafe_set` [*Type*], specifying the type that will be unsafely converted into the raw value on writes, using `UnsafeInto<T>`
    - `unsafe_both` [*Type*], as shorthand for `unsafe_get` [*Type*] and `unsafe_set` [*Type*]
    - `unsafe` [*Type*], as shorthand for `unsafe_get` [*Type*] and `set` [*Type*]

# The `bits!`, `with_bits!` and `set_bits!` macros

These macros provide simplified bitfield functionality without the need to declare a bitfield struct: the value serving as an anonymous bitfield is provided as their first argument, followed by the [*FieldRange*] to access analogously to the fields declarations in the `bitfield!` macro, i.e. `bits!(0x1234_u16, 0..=15)`.

For the `with_bits!` and `set_bits!` macros, a new value for the field is provided by appending `= value` to the field bit range specification, i.e. `with_bits!(0x1234, 0..4 = 0xF)`, `set_bits!(b, 15 = true)`.

## Specifying bitfield and field types

In cases where type inference fails, the accessed field's type `T` can be specified by prepending `T @` to the [*FieldRange*] specification, i.e. `bits!(0x1234_u16, u8 @ 0..=7)`. The macros can also detect simple `as` casts in the provided expressions (for the bitfield's value in all cases, and for the field's new value for `with_bits!` and `set_bits!`) and treat them as explicit type specifications.

Due to implementation limitations, specifying the bitfield's storage type through a cast is required when the field's bit range is `..`, i.e. `bits!(0x1234 as u16, ..)`.

An explicit field type mustn't be specified when accessing a single bit as a boolean (using the single [*LiteralExpression*] form of [*FieldRange*]), as analogously to `bitfield!` fields it's always fixed to `bool`.

## Formal syntax

The general formal syntax for macro calls is:

`bits!`:
> [*Expression*] `,` ([*Type*]`@`)<sup>?</sup> *FieldRange*

`with_bits!` and `set_bits!`:
> [*Expression*] `,` ([*Type*]`@`)<sup>?</sup> *FieldRange* `=` [*Expression*]

# Other derive macros

The crate provides other supporting derive macros associated with bitfield functionality.

## `ConvRaw`

`ConvRaw` is a derive macro to implement any applicable conversion traits between a non-empty fieldless enum and the builtin integer types corresponding to variant discriminants.

It will implement `TryFrom<T> for Enum` for all builtin integer types `T`, and `From<Enum> for T` for all types that can fit all the enum discriminants.

If the enum only contains two variants with discriminants 0 and 1 (in any order), it will also implement `From<bool> for Enum` and `From<Enum> for bool`.

## `UnwrapBits`

`UnwrapBits` is a derive macro to implement `Bits<T> for U`, `WithBits<T> for U` and `SetBits<T> for U` for a type `T` and all builtin integer types `U` used as bitfield storage types.

For each integer type `U`, an implementation will be generated iff `T: TryFrom<U> + Into<U>` and `<T as TryFrom<U>>::Error: Debug`, and unwraps the result of `<T as TryFrom<U>>::try_from` to convert the field's raw integer value to `T` on reads.

This derive macro is currently gated behind the `nightly` feature, as it requires [`#![feature(trivial_bounds)]`](https://doc.rust-lang.org/beta/unstable-book/language-features/trivial-bounds.html) to be enabled in the crate using it.

[*FieldRange*]: #field-declarations
[*Visibility*]: https://doc.rust-lang.org/stable/reference/visibility-and-privacy.html
[IDENTIFIER]: https://doc.rust-lang.org/stable/reference/identifiers.html
[*Type*]: https://doc.rust-lang.org/stable/reference/types.html#type-expressions
[*Expression*]: https://doc.rust-lang.org/stable/reference/expressions.html
[*LiteralExpression*]: https://doc.rust-lang.org/stable/reference/expressions/literal-expr.html
