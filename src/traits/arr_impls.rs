use super::{Bit, Bits, SetBit, SetBits, WithBit, WithBits};

macro_rules! impl_bits_for_int_type {
    ($storage: ident ($storage_u: ident), $value: ident ($value_u: ident) $(, $const:tt)?) => {
        mod $value {
            use super::*;
            const S_SHIFT: u32 = <$storage>::BITS.trailing_zeros();
            const S_MASK: usize = <$storage>::BITS as usize - 1;
            const V_BITS: usize = <$value>::BITS as usize;
            const V_SHIFT: u32 = <$value>::BITS.trailing_zeros();

            impl<const N: usize> $($const)* Bits<$value> for [$storage; N] {
                #[inline]
                fn bits<const START: usize, const END: usize>(&self) -> $value {
                    if START >= END {
                        return 0;
                    }
                    let bits = if START >> S_SHIFT == (END - 1) >> S_SHIFT {
                        (self[START >> S_SHIFT] >> (START & S_MASK)) as $value
                    } else {
                        let mut bits: $value = 0;
                        let mut i = START >> S_SHIFT;
                        while i <= (END - 1) >> S_SHIFT {
                            let start = $crate::__private::max(START, i << S_SHIFT);
                            bits |= ((self[i] as $storage_u >> (start & S_MASK)) as $value)
                                << (start - START);
                            i += 1;
                        }
                        bits
                    };
                    let read_bits = END - START;
                    bits << (V_BITS - read_bits) >> (V_BITS - read_bits)
                }
            }

            impl<const N: usize> $($const)* WithBits<$value> for [$storage; N] {
                #[inline]
                fn with_bits<const START: usize, const END: usize>(
                    mut self,
                    value: $value,
                ) -> Self {
                    self.set_bits::<START, END>(value);
                    self
                }
            }

            impl<const N: usize> $($const)* SetBits<$value> for [$storage; N] {
                #[inline]
                fn set_bits<const START: usize, const END: usize>(&mut self, value: $value) {
                    if START >= END {
                        return;
                    }
                    if START >> S_SHIFT == (END - 1) >> S_SHIFT {
                        let i = START >> S_SHIFT;
                        let written_bits = END - START;
                        let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1)
                            << (START & S_MASK);
                        self[i] =
                            (self[i] & !mask) | ((value as $storage) << (START & S_MASK) & mask);
                    } else {
                        let mut i = START >> S_SHIFT;
                        while i <= (END - 1) >> S_SHIFT {
                            let start = $crate::__private::max(START, i << S_SHIFT);
                            let end = $crate::__private::min(END, (i + 1) << S_SHIFT);
                            let written_bits = end - start;
                            let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1)
                                << (start & S_MASK);
                            self[i] = (self[i] & !mask)
                                | (((value >> (start - START)) as $storage) << (start & S_MASK)
                                    & mask);
                            i += 1;
                        }
                    }
                }
            }

            impl $($const)* Bits<$value> for [$storage] {
                #[inline]
                fn bits<const START: usize, const END: usize>(&self) -> $value {
                    if START >= END {
                        return 0;
                    }
                    let bits = if START >> S_SHIFT == (END - 1) >> S_SHIFT {
                        (self[START >> S_SHIFT] >> (START & S_MASK)) as $value
                    } else {
                        let mut bits: $value = 0;
                        let mut i = START >> S_SHIFT;
                        while i <= (END - 1) >> S_SHIFT {
                            let start = $crate::__private::max(START, i << S_SHIFT);
                            bits |= ((self[i] as $storage_u >> (start & S_MASK)) as $value)
                                << (start - START);
                            i += 1;
                        }
                        bits
                    };
                    let read_bits = END - START;
                    bits << (V_BITS - read_bits) >> (V_BITS - read_bits)
                }
            }

            impl $($const)* SetBits<$value> for [$storage] {
                #[inline]
                fn set_bits<const START: usize, const END: usize>(&mut self, value: $value) {
                    if START >= END {
                        return;
                    }
                    if START >> S_SHIFT == (END - 1) >> S_SHIFT {
                        let i = START >> S_SHIFT;
                        let written_bits = END - START;
                        let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1)
                            << (START & S_MASK);
                        self[i] =
                            (self[i] & !mask) | ((value as $storage) << (START & S_MASK) & mask);
                    } else {
                        let mut i = START >> S_SHIFT;
                        while i <= (END - 1) >> S_SHIFT {
                            let start = $crate::__private::max(START, i << S_SHIFT);
                            let end = $crate::__private::min(END, (i + 1) << S_SHIFT);
                            let written_bits = end - start;
                            let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1)
                                << (start & S_MASK);
                            self[i] = (self[i] & !mask)
                                | (((value >> (start - START)) as $storage) << (start & S_MASK)
                                    & mask);
                            i += 1;
                        }
                    }
                }
            }

            impl<const M: usize> $($const)* Bits<[$value; M]> for $storage {
                #[inline]
                fn bits<const START: usize, const END: usize>(&self) -> [$value; M] {
                    if START >= END {
                        return [0; M];
                    }
                    let mut result = [0; M];
                    let mut i = 0;
                    while i <= (END - START - 1) >> V_SHIFT {
                        let start = START + (i << V_SHIFT);
                        let end = $crate::__private::min(END, start + V_BITS);
                        let read_bits = end - start;
                        result[i] = ((*self >> start) as $value) << (V_BITS - read_bits)
                            >> (V_BITS - read_bits);
                        i += 1;
                    }
                    result
                }
            }

            impl<const M: usize> $($const)* WithBits<[$value; M]> for $storage {
                #[inline]
                fn with_bits<const START: usize, const END: usize>(
                    mut self,
                    value: [$value; M],
                ) -> Self {
                    if START >= END {
                        return self;
                    }
                    let mut i = 0;
                    while i <= (END - START - 1) >> V_SHIFT {
                        let start = START + (i << V_SHIFT);
                        let end = $crate::__private::min(END, start + V_BITS);
                        let written_bits = end - start;
                        let mask =
                            ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1) << start;
                        self = (self & !mask) | ((value[i] as $storage) << start & mask);
                        i += 1;
                    }
                    self
                }
            }

            impl<const M: usize> $($const)* SetBits<[$value; M]> for $storage {
                #[inline]
                fn set_bits<const START: usize, const END: usize>(&mut self, value: [$value; M]) {
                    *self = self.with_bits::<START, END>(value);
                }
            }

            impl<const M: usize, const N: usize> $($const)* Bits<[$value; M]> for [$storage; N] {
                #[inline]
                fn bits<const START: usize, const END: usize>(&self) -> [$value; M] {
                    if START >= END {
                        return [0; M];
                    }
                    let mut result = [0; M];
                    let mut i = 0;
                    while i <= (END - START - 1) >> V_SHIFT {
                        let start = START + (i << V_SHIFT);
                        let end = $crate::__private::min(END, start + V_BITS);
                        let bits = if start >> S_SHIFT == (end - 1) >> S_SHIFT {
                            (self[start >> S_SHIFT] >> (start & S_MASK)) as $value
                        } else {
                            let mut bits: $value = 0;
                            let mut j = start >> S_SHIFT;
                            while j <= (end - 1) >> S_SHIFT {
                                let start_ = $crate::__private::max(start, j << S_SHIFT);
                                bits |= ((self[j] as $storage_u >> (start_ & S_MASK)) as $value)
                                    << (start_ - start);
                                j += 1;
                            }
                            bits
                        };
                        let read_bits = end - start;
                        result[i] = bits << (V_BITS - read_bits) >> (V_BITS - read_bits);
                        i += 1;
                    }
                    result
                }
            }

            impl<const M: usize, const N: usize> $($const)* WithBits<[$value; M]> for
                [$storage; N]
            {
                #[inline]
                fn with_bits<const START: usize, const END: usize>(
                    mut self,
                    value: [$value; M],
                ) -> Self {
                    self.set_bits::<START, END>(value);
                    self
                }
            }

            impl<const M: usize, const N: usize> $($const)* SetBits<[$value; M]> for [$storage; N] {
                #[inline]
                fn set_bits<const START: usize, const END: usize>(&mut self, value: [$value; M]) {
                    if START >= END {
                        return;
                    }
                    let mut i = 0;
                    while i <= (END - START - 1) >> V_SHIFT {
                        let start = START + (i << V_SHIFT);
                        let end = $crate::__private::min(END, start + V_BITS);
                        if start >> S_SHIFT == (end - 1) >> S_SHIFT {
                            let j = start >> S_SHIFT;
                            let written_bits = end - start;
                            let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1)
                                << (start & S_MASK);
                            self[j] = (self[j] & !mask)
                                | ((value[i] as $storage) << (start & S_MASK) & mask);
                        } else {
                            let mut j = start >> S_SHIFT;
                            while j <= (end - 1) >> S_SHIFT {
                                let start_ = $crate::__private::max(start, j << S_SHIFT);
                                let end_ = $crate::__private::min(end, (j + 1) << S_SHIFT);
                                let written_bits = end_ - start_;
                                let mask = ((1 as $storage) << (written_bits - 1) << 1)
                                    .wrapping_sub(1)
                                    << (start_ & S_MASK);
                                self[j] = (self[j] & !mask)
                                    | (((value[i] >> (start_ - start)) as $storage)
                                        << (start_ & S_MASK)
                                        & mask);
                                j += 1;
                            }
                        }
                        i += 1;
                    }
                }
            }

            impl<const M: usize> $($const)* Bits<[$value; M]> for [$storage] {
                #[inline]
                fn bits<const START: usize, const END: usize>(&self) -> [$value; M] {
                    if START >= END {
                        return [0; M];
                    }
                    let mut result = [0; M];
                    let mut i = 0;
                    while i <= (END - START - 1) >> V_SHIFT {
                        let start = START + (i << V_SHIFT);
                        let end = $crate::__private::min(END, start + V_BITS);
                        let bits = if start >> S_SHIFT == (end - 1) >> S_SHIFT {
                            (self[start >> S_SHIFT] >> (start & S_MASK)) as $value
                        } else {
                            let mut bits: $value = 0;
                            let mut j = start >> S_SHIFT;
                            while j <= (end - 1) >> S_SHIFT {
                                let start_ = $crate::__private::max(start, j << S_SHIFT);
                                bits |= ((self[j] as $storage_u >> (start_ & S_MASK)) as $value)
                                    << (start_ - start);
                                j += 1;
                            }
                            bits
                        };
                        let read_bits = end - start;
                        result[i] = bits << (V_BITS - read_bits) >> (V_BITS - read_bits);
                        i += 1;
                    }
                    result
                }
            }

            impl<const M: usize> $($const)* SetBits<[$value; M]> for [$storage] {
                #[inline]
                fn set_bits<const START: usize, const END: usize>(&mut self, value: [$value; M]) {
                    if START >= END {
                        return;
                    }
                    let mut i = 0;
                    while i <= (END - START - 1) >> V_SHIFT {
                        let start = START + (i << V_SHIFT);
                        let end = $crate::__private::min(END, start + V_BITS);
                        if start >> S_SHIFT == (end - 1) >> S_SHIFT {
                            let j = start >> S_SHIFT;
                            let written_bits = end - start;
                            let mask = ((1 as $storage) << (written_bits - 1) << 1).wrapping_sub(1)
                                << (start & S_MASK);
                            self[j] = (self[j] & !mask)
                                | ((value[i] as $storage) << (start & S_MASK) & mask);
                        } else {
                            let mut j = start >> S_SHIFT;
                            while j <= (end - 1) >> S_SHIFT {
                                let start_ = $crate::__private::max(start, j << S_SHIFT);
                                let end_ = $crate::__private::min(end, (j + 1) << S_SHIFT);
                                let written_bits = end_ - start_;
                                let mask = ((1 as $storage) << (written_bits - 1) << 1)
                                    .wrapping_sub(1)
                                    << (start_ & S_MASK);
                                self[j] = (self[j] & !mask)
                                    | (((value[i] >> (start_ - start)) as $storage)
                                        << (start_ & S_MASK)
                                        & mask);
                                j += 1;
                            }
                        }
                        i += 1;
                    }
                }
            }
        }
    };
}

macro_rules! impl_bits_for_int_types {
    (const => $($dst_ty: ident ($dst_u_ty: ident)),*) => {};
    (=> $($dst_ty: ident ($dst_u_ty: ident)),*) => {};
    (
        const $src_ty: ident ($src_u_ty: ident)
        $(, $other_src_ty: ident ($other_src_u_ty: ident))*
        => $($dst_ty: ident ($dst_u_ty: ident)),*
    ) => {
        mod $src_ty {
            use super::*;
            $(
                impl_bits_for_int_type!($src_ty ($src_u_ty), $dst_ty ($dst_u_ty), const);
            )*
        }
        impl_bits_for_int_types!(
            const $($other_src_ty ($other_src_u_ty)),* => $($dst_ty ($dst_u_ty)),*
        );
    };
    (
        $src_ty: ident ($src_u_ty: ident)
        $(, $other_src_ty: ident ($other_src_u_ty: ident))*
        => $($dst_ty: ident ($dst_u_ty: ident)),*
    ) => {
        mod $src_ty {
            use super::*;
            $(
                impl_bits_for_int_type!($src_ty ($src_u_ty), $dst_ty ($dst_u_ty));
            )*
        }
        impl_bits_for_int_types!(
            $($other_src_ty ($other_src_u_ty)),* => $($dst_ty ($dst_u_ty)),*
        );
    };
}

mod bits {
    use super::*;
    #[cfg(not(feature = "nightly"))]
    impl_bits_for_int_types!(
        u8 (u8), u16 (u16), u32 (u32), u64 (u64), u128 (u128), usize (usize),
        i8 (u8), i16 (u16), i32 (u32), i64 (u64), i128 (u128), isize (usize)
            => u8 (u8), u16 (u16), u32 (u32), u64 (u64), u128 (u128), usize (usize),
               i8 (u8), i16 (u16), i32 (u32), i64 (u64), i128 (u128), isize (usize)
    );
    #[cfg(feature = "nightly")]
    impl_bits_for_int_types!(
        const
        u8 (u8), u16 (u16), u32 (u32), u64 (u64), u128 (u128), usize (usize),
        i8 (u8), i16 (u16), i32 (u32), i64 (u64), i128 (u128), isize (usize)
            => u8 (u8), u16 (u16), u32 (u32), u64 (u64), u128 (u128), usize (usize),
               i8 (u8), i16 (u16), i32 (u32), i64 (u64), i128 (u128), isize (usize)
    );
}

macro_rules! impl_bit_for_arr_int_type {
    ($t: ident $(, $const:tt)?) => {
        mod $t {
            use super::*;
            const SHIFT: u32 = <$t>::BITS.trailing_zeros();
            const MASK: usize = <$t>::BITS as usize - 1;

            impl<const N: usize> $($const)* Bit for [$t; N] {
                #[inline]
                fn bit<const BIT: usize>(&self) -> bool {
                    self[BIT >> SHIFT] & 1 << (BIT & MASK) != 0
                }
            }

            impl<const N: usize> $($const)* WithBit for [$t; N] {
                #[inline]
                fn with_bit<const BIT: usize>(mut self, value: bool) -> Self {
                    self.set_bit::<BIT>(value);
                    self
                }
            }

            impl<const N: usize> $($const)* SetBit for [$t; N] {
                #[inline]
                fn set_bit<const BIT: usize>(&mut self, value: bool) {
                    self[BIT >> SHIFT] =
                        (self[BIT >> SHIFT] & !(1 << (BIT & MASK))) | (value as $t) << BIT;
                }
            }

            impl $($const)* Bit for [$t] {
                #[inline]
                fn bit<const BIT: usize>(&self) -> bool {
                    self[BIT >> SHIFT] & 1 << (BIT & MASK) != 0
                }
            }

            impl $($const)* SetBit for [$t] {
                #[inline]
                fn set_bit<const BIT: usize>(&mut self, value: bool) {
                    self[BIT >> SHIFT] =
                        (self[BIT >> SHIFT] & !(1 << (BIT & MASK))) | (value as $t) << BIT;
                }
            }
        }
    };
}

macro_rules! impl_bit_for_arr_int_types {
    (const $($t: ident),*) => {
        $(impl_bit_for_arr_int_type!($t, const);)*
    };
    ($($t: ident),*) => {
        $(impl_bit_for_arr_int_type!($t);)*
    };
}

mod bit {
    use super::*;
    #[cfg(not(feature = "nightly"))]
    impl_bit_for_arr_int_types!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    #[cfg(feature = "nightly")]
    impl_bit_for_arr_int_types!(
        const
        u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
    );
}
