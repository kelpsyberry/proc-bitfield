use crate::{Bitfield, Bits, SetBits};
use core::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct NestedRef<'a, T: Bitfield, U: Bitfield, const START: usize, const END: usize>
where
    T::Storage: Bits<U::Storage>,
{
    value: U,
    _parent: PhantomData<&'a T>,
}

impl<T: Bitfield, U: Bitfield, const START: usize, const END: usize> fmt::Debug
    for NestedRef<'_, T, U, START, END>
where
    T::Storage: Bits<U::Storage>,
    U: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl<'a, T: Bitfield, U: Bitfield, const START: usize, const END: usize>
    NestedRef<'a, T, U, START, END>
where
    T::Storage: Bits<U::Storage>,
{
    #[inline]
    pub fn new(parent: &'a T) -> Self {
        NestedRef {
            value: U::from_storage(parent.storage().bits::<START, END>()),
            _parent: PhantomData,
        }
    }
}

impl<T: Bitfield, U: Bitfield, const START: usize, const END: usize> Deref
    for NestedRef<'_, T, U, START, END>
where
    T::Storage: Bits<U::Storage>,
{
    type Target = U;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub struct NestedRefMut<'a, T: Bitfield, U: Bitfield, const START: usize, const END: usize>
where
    T::Storage: Bits<U::Storage> + SetBits<U::Storage>,
    U::Storage: Clone,
{
    value: U,
    parent: &'a mut T,
}

impl<'a, T: Bitfield, U: Bitfield, const START: usize, const END: usize>
    NestedRefMut<'a, T, U, START, END>
where
    T::Storage: Bits<U::Storage> + SetBits<U::Storage>,
    U::Storage: Clone,
{
    #[inline]
    pub fn new(parent: &'a mut T) -> Self {
        NestedRefMut {
            value: U::from_storage(parent.storage().bits::<START, END>()),
            parent,
        }
    }
}

impl<T: Bitfield, U: Bitfield, const START: usize, const END: usize> Deref
    for NestedRefMut<'_, T, U, START, END>
where
    T::Storage: Bits<U::Storage> + SetBits<U::Storage>,
    U::Storage: Clone,
{
    type Target = U;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Bitfield, U: Bitfield, const START: usize, const END: usize> DerefMut
    for NestedRefMut<'_, T, U, START, END>
where
    T::Storage: Bits<U::Storage> + SetBits<U::Storage>,
    U::Storage: Clone,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Bitfield, U: Bitfield, const START: usize, const END: usize> Drop
    for NestedRefMut<'_, T, U, START, END>
where
    T::Storage: Bits<U::Storage> + SetBits<U::Storage>,
    U::Storage: Clone,
{
    #[inline]
    fn drop(&mut self) {
        self.parent
            .storage_mut()
            .set_bits::<START, END>(self.value.storage().clone());
    }
}
