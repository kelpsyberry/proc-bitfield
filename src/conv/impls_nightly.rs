use super::{Try, UnsafeFrom, UnsafeInto};

impl<T> const Try for Option<T> {
    type Output = T;
    type WithOutput<U> = Option<U>;

    fn from_output(output: Self::Output) -> Self {
        Some(output)
    }
}

impl<T, E> const Try for Result<T, E> {
    type Output = T;
    type WithOutput<U> = Result<U, E>;

    fn from_output(output: Self::Output) -> Self {
        Ok(output)
    }
}

impl<T, U> UnsafeFrom<U> for T
where
    U: Into<T>,
{
    /// Calls `U::into(other)`.
    ///
    /// That is, this conversion is whatever the implementation of [`Into`]`<T> for U` chooses to
    /// do.
    #[inline]
    unsafe fn unsafe_from(other: U) -> Self {
        U::into(other)
    }
}

impl<T, U> const UnsafeInto<U> for T
where
    U: [const] UnsafeFrom<T>,
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
