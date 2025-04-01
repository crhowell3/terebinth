pub trait DebugStrictAdd {
    /// See [`DebugStrictAdd`].
    fn debug_strict_add(self, other: Self) -> Self;
}

macro_rules! impl_debug_strict_add {
    ($( $ty:ty )*) => {
        $(
            impl DebugStrictAdd for $ty {
                fn debug_strict_add(self, other: Self) -> Self {
                    if cfg!(debug_assertions) {
                        self + other
                    } else {
                        self.wrapping_add(other)
                    }
                }
            }
        )*
    };
}

/// See [`DebugStrictAdd`].
pub trait DebugStrictSub {
    /// See [`DebugStrictAdd`].
    fn debug_strict_sub(self, other: Self) -> Self;
}

macro_rules! impl_debug_strict_sub {
    ($( $ty:ty )*) => {
        $(
            impl DebugStrictSub for $ty {
                fn debug_strict_sub(self, other: Self) -> Self {
                    if cfg!(debug_assertions) {
                        self - other
                    } else {
                        self.wrapping_sub(other)
                    }
                }
            }
        )*
    };
}

impl_debug_strict_add! {
    u8 u16 u32 u64 u128 usize
    i8 i16 i32 i64 i128 isize
}

impl_debug_strict_sub! {
    u8 u16 u32 u64 u128 usize
    i8 i16 i32 i64 i128 isize
}
