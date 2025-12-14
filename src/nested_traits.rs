pub const trait NestedBitfield<'a, S> {
    fn __from_storage(storage: &'a S) -> Self;
}

pub const trait NestedMutBitfield<'a, S> {
    fn __from_storage(storage: &'a mut S) -> Self;
}
