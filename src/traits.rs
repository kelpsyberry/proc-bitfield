mod arr_impls;
mod int_impls;

/// Read a range of bits inside a value.
pub trait Bits<T> {
    /// Read `self`'s `START..END` bit range (with `END` excluded) as a value of type `T`.
    fn bits<const START: usize, const END: usize>(&self) -> T;
}

/// Return a value with a range of bits modified.
pub trait WithBits<T> {
    #[must_use]
    /// Returns `self` with the the `START..END` bit range (with `END` excluded) set to the given
    /// value of type `T`.
    fn with_bits<const START: usize, const END: usize>(self, value: T) -> Self;
}

/// Modify a range of bits inside a value in place.
pub trait SetBits<T> {
    /// Sets `self`'s `START..END` bit range (with `END` excluded) to the given value of type `T`
    /// in place.
    fn set_bits<const START: usize, const END: usize>(&mut self, value: T);
}

/// Read a single bit inside a value.
pub trait Bit {
    /// Read `self`'s specified bit.
    fn bit<const BIT: usize>(&self) -> bool;
}

/// Return a value with a single bit modified.
pub trait WithBit {
    /// Returns `self` with the the specified bit set to 1 if `value` is `true`, and 0 otherwise.
    #[must_use]
    fn with_bit<const BIT: usize>(self, value: bool) -> Self;
}

/// Modify a single bit inside a value in place.
pub trait SetBit {
    /// Sets `self`'s specified bit to 1 if `value` is `true`, and 0 otherwise.
    fn set_bit<const BIT: usize>(&mut self, value: bool);
}
