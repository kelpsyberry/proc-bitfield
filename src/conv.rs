#![allow(clippy::missing_safety_doc)]

#[cfg(feature = "nightly")]
mod const_impls;
#[cfg(not(feature = "nightly"))]
mod impls;

/// Unsafe equivalent of [`From`].
///
/// Used to do unsafe value-to-value conversions while consuming the input value. It is the
/// reciprocal of [`UnsafeInto`].
///
/// One should always prefer implementing `UnsafeFrom` over [`UnsafeInto`] because implementing
/// `UnsafeFrom` automatically provides one with an implementation of [`UnsafeInto`] thanks to the
/// blanket implementation.
///
/// Prefer using [`UnsafeInto`] over using `UnsafeFrom` when specifying trait bounds on a generic
/// function. This way, types that directly implement [`UnsafeInto`] can be used as arguments as
/// well.
///
/// **Note: This trait must not fail**. The `UnsafeFrom` trait is intended for unsafe but perfect
/// conversions. If the conversion can fail or is not perfect, use [`TryFrom`].
///
/// # Generic Implementations
///
/// - `UnsafeFrom<T> for U` implies [`UnsafeInto`]`<U> for T`
/// - `UnsafeFrom` is reflexive, which means that `UnsafeFrom<T> for T` is implemented
/// - [`From`]`<T> for U` implies `UnsafeFrom<T> for U`
pub trait UnsafeFrom<T> {
    /// Unsafely converts to this type from the input type.
    unsafe fn unsafe_from(_: T) -> Self;
}

/// Unsafe equivalent of [`From`].
///
/// Used to do unsafe value-to-value conversions while consuming the input value. It is the
/// reciprocal of [`UnsafeInto`].
///
/// One should always prefer implementing `UnsafeFrom` over [`UnsafeInto`] because implementing
/// `UnsafeFrom` automatically provides one with an implementation of [`UnsafeInto`] thanks to the
/// blanket implementation.
///
/// Prefer using [`UnsafeInto`] over using `UnsafeFrom` when specifying trait bounds on a generic
/// function. This way, types that directly implement [`UnsafeInto`] can be used as arguments as
/// well.
///
/// **Note: This trait must not fail**. The `UnsafeFrom` trait is intended for unsafe but perfect
/// conversions. If the conversion can fail or is not perfect, use [`TryFrom`].
///
/// # Generic Implementations
///
/// - `UnsafeFrom<T> for U` implies [`UnsafeInto`]`<U> for T`
/// - `UnsafeFrom` is reflexive, which means that `UnsafeFrom<T> for T` is implemented
/// - [`From`]`<T> for U` implies `UnsafeFrom<T> for U`
pub trait UnsafeInto<T> {
    /// Unsafely converts this type into the input type.
    unsafe fn unsafe_into(self) -> T;
}
