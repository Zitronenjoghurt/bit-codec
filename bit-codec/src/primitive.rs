pub trait BitInt: Copy {
    const BITS: u32;
    const ZERO: Self;
    /// Low 8 bits of `self >> shift`. Caller guarantees `shift < Self::BITS`.
    fn extract_byte(self, shift: u32) -> u8;
    /// `self | ((bits as Self) << shift)`. Caller guarantees `shift < Self::BITS`.
    fn deposit_byte(self, bits: u8, shift: u32) -> Self;
}

macro_rules! impl_bit_int {
    ($($t:ty),*) => {$(
        impl BitInt for $t {
            const BITS: u32 = <$t>::BITS;
            const ZERO: Self = 0;
            #[inline(always)]
            fn extract_byte(self, shift: u32) -> u8 {
                debug_assert!(shift < Self::BITS);
                (self >> shift) as u8
            }
            #[inline(always)]
            fn deposit_byte(self, bits: u8, shift: u32) -> Self {
                debug_assert!(shift < Self::BITS);
                self | ((bits as $t) << shift)
            }
        }
    )*};
}
impl_bit_int!(u8, u16, u32, u64, u128, usize);
