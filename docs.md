# `proc-bitfield`

A Rust crate to expressively declare bitfield-like `struct`s, automatically ensuring their correctness at compile time and declaring accessors.

## Automatic `Debug` implementation

A `fmt::Debug` implementation can be implemented automatically for a given bitfield struct by adding `: Debug` after the tuple struct-like storage type declaration; the generated `fmt` function will output the type's raw value as well as all of its fields' values.

## `nightly` feature

Optionally, the `nightly` feature can be enabled to use `const Trait` functionality: this makes the `BitRange` and `Bit` traits be implemented using `const fn`s for all integer types, and enables the option to use `const fn`s for field accessors.

With the feature enabled, `const fn` accessors can be enabled globally for a struct by replacing `struct` with `const struct` (i.e. `const struct Example(pub u16)`), or on a field-by-field basis by prepending `const` to its type (i.e. `raw: const u16 @ ..`).

## Field declarations

Fields can be declared by using the form:
> [*Visibility*] [IDENTIFIER] `:` [*Type*] (`[`(*Option* `,`)<sup>*</sup> *Option*`]`)<sup>?</sup> `@` *FieldRange*

where *FieldRange* corresponds to any of (where *L* is an alias for *LiteralExpression*):
- `..`, to use every bit
- *L*`..=`*L*, to use the bits specified by an inclusive range
- *L*`..`*L*, to use the bits specified by an exclusive range
- *L*`;` *L*, to use bits specified by a (start, length) pair
- *L*, to use a single bit; unlike all other specifications, this is only valid for `bool` fields, and will use the `Bit` trait instead of `BitRange`

*Option*s can be optionally specified in brackets, matching any of the ones defined below.

### Access restrictions

Fields are both readable and writable by default, but can be declared read-only or write-only using respectively the `read_only`/`ro` and `write_only`/`wo` options.

### Field type conversions

Fields' "raw" types as specified after the colon are restricted by `BitRange<T>` implementations on the bitfield's contained type; however, accessors can perform conversions specified through optional options. These can be:
- Infallible conversions, using the `From<T>` and `Into<T>` traits, the relevant options being:
    - `get` [*Type*], specifying the type that the raw value will be converted into on reads, using `From<T>`
    - `set` [*Type*], specifying the type that will be converted into the raw value on writes, using `Into<T>`
    - [*Type*], as a shorthand for `get` [*Type*] and `set` [*Type*]
- Fallible conversions, using the `TryFrom<T>` and `TryInto<T>` traits, the relevant options being:
    - `try_get` [*Type*], specifying the type that the raw value will be fallibly converted into on reads, using `TryFrom<T>`
    - `try_set` [*Type*], specifying the type that will be fallibly converted into the raw value on writes, using `TryInto<T>`
    - `try_both` [*Type*], as a shorthand for `try_get` [*Type*] and `try_set` [*Type*]
    - `try` [*Type*], as a shorthand for `try_get` [*Type*] and `set` [*Type*]
- Unsafe conversions, using the `UnsafeFrom<T>` and `UnsafeInto<T>` traits, the relevant options being:
    - `unsafe_get` [*Type*], specifying the type that the raw value will be unsafely converted into on reads, using `UnsafeFrom<T>`
    - `unsafe_set` [*Type*], specifying the type that will be unsafely converted into the raw value on writes, using `UnsafeInto<T>`
    - `unsafe_both` [*Type*], as shorthand for `unsafe_get` [*Type*] and `unsafe_set` [*Type*]
    - `unsafe` [*Type*], as shorthand for `unsafe_get` [*Type*] and `set` [*Type*]

[*Visibility*]: https://doc.rust-lang.org/stable/reference/visibility-and-privacy.html
[IDENTIFIER]: https://doc.rust-lang.org/stable/reference/identifiers.html
[*Type*]: https://doc.rust-lang.org/stable/reference/types.html#type-expressions
