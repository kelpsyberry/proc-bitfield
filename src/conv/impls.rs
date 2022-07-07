use super::{UnsafeFrom, UnsafeInto};

impl<T, U> UnsafeFrom<U> for T
where
    T: From<U>,
{
    /// Calls `T::from(other)`.
    ///
    /// That is, this conversion is whatever the implementation of [`From`]`<U> for T` chooses to
    /// do.
    #[inline]
    unsafe fn unsafe_from(other: U) -> Self {
        Self::from(other)
    }
}

impl<T, U> UnsafeInto<U> for T
where
    U: UnsafeFrom<T>,
{
    /// Calls `U::unsafe_from(self)`.
    ///
    /// That is, this conversion is whatever the implementation of [`UnsafeFrom`]`<T> for U`
    /// chooses to do.
    #[inline]
    unsafe fn unsafe_into(self) -> U {
        U::unsafe_from(self)
    }
}
