#![cfg_attr(all(feature = "nightly", test), feature(stmt_expr_attributes))]
#![cfg_attr(feature = "nightly", allow(internal_features))]
#![cfg_attr(feature = "nightly", feature(extend_one, step_trait, test))]
#![cfg_attr(feature = "nightly", feature(new_range_api))]
#![cfg_attr(feature = "nightly", feature(new_zeroed_alloc))]

pub mod bit_set;
pub mod interval;

mod idx;
mod slice;
mod vec;

pub use idx::{Idx, IntoSliceIdx};
pub use slice::IndexSlice;
pub use terebinth_index_macros::newtype_index;
pub use vec::IndexVec;

#[macro_export]
#[cfg(not(feature = "rustc_randomized_layouts"))]
macro_rules! static_assert_size {
    ($ty:ty, $size:expr) => {
        const _: [(); $size] = [(); ::std::mem::size_of::<$ty>()];
    };
}

#[macro_export]
#[cfg(feature = "rustc_randomized_layouts")]
macro_rules! static_assert_size {
    ($ty:ty, $size:expr) => {
        const _: (usize, usize) = ($size, ::std::mem::size_of::<$ty>());
    };
}
