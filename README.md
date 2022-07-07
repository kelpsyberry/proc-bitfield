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

## Automatic `Debug` implementation

A `fmt::Debug` implementation can be implemented automatically for a given bitfield struct by adding `: Debug` after the tuple struct-like storage type declaration; the generated `fmt` function will output the type's raw value as well as all of its fields' values.

## `nightly` feature

Optionally, the `nightly` feature can be enabled to use `const Trait` functionality: this makes the `BitRange` and `Bit` traits be implemented using `const fn`s for all integer types, and enables the option to use `const fn`s for field accessors.

With the feature enabled, `const fn` accessors can be enabled globally for a struct by replacing `struct` with `const struct` (i.e. `const struct Example(pub u16)`), or on a field-by-field basis by prepending `const` to its type (i.e. `raw: const u16 @ ..`).

## Field declarations

Fields can be declared by using the form:
> [*Visibility*] [IDENTIFIER] `:` [*Type*] (`[`(*Attribute* `,`)<sup>*</sup> *Attribute*`]`)<sup>?</sup> `@` *FieldRange*

where *FieldRange* corresponds to any of (where *L* is an alias for *LiteralExpression*):
- `..`, to use every bit
- *L*`..=`*L*, to use the bits specified by an inclusive range
- *L*`..`*L*, to use the bits specified by an exclusive range
- *L*`;` *L*, to use bits specified by a (start, length) pair
- *L*, to use a single bit; unlike all other specifications, this is only valid for `bool` fields, and will use the `Bit` trait instead of `BitRange`

*Attribute*s can be optionally specified in brackets, matching any of the ones defined below.

### Access restrictions

Fields are both readable and writable by default, but can be declared read-only or write-only using respectively the `read_only` and `write_only` attributes.

### Field type conversions

Fields' "raw" types as specified after the colon are restricted by `BitRange` implementations on the bitfield's contained type; however, accessors can perform conversions specified through optional attributes. These can be:
- Infallible conversions, using the `From` and `Into` traits, the relevant attributes being:
    - `get` *Type*, specifying the type that the raw value will be converted into using `From<T>` for reads
    - `set` *Type*, specifying the type that will be converted into the raw value using `Into<T>` for writes
    - *Type*, as shorthand for `get` *Type* and `set` *Type*
- Fallible conversions, using the `TryFrom` and `TryInto` traits, the relevant attributes being:
    - `try_get` *Type*, specifying the type that the raw value will be fallibly converted into using `TryFrom<T>` for reads
    - `try_set` *Type*, specifying the type that will be fallibly converted into the raw value into using `TryInto<T>` for writes
    - `try` *Type*, as shorthand for `try_get` *Type* and `try_set` *Type*
- Unsafe (for reads) conversions, using the `UnsafeFrom` and `Into` traits, the relevant attributes being:
    - `unsafe_get` *Type*, specifying the type that the raw value will be unsafely converted into using `UnsafeFrom<T>` for reads
    - `unsafe` *Type*, as shorthand for `unsafe_get` *Type* and `set` *Type*

[*Visibility*]: https://doc.rust-lang.org/stable/reference/visibility-and-privacy.html
[IDENTIFIER]: https://doc.rust-lang.org/stable/reference/identifiers.html
[*Type*]: https://doc.rust-lang.org/stable/reference/types.html#type-expressions
