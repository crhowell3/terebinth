#![allow(internal_features)]
#![allow(rustc::default_hash_types)]
#![allow(rustc::potential_query_instability)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(allocator_api)]
#![feature(array_windows)]
#![feature(ascii_char)]
#![feature(ascii_char_variants)]
#![feature(assert_matches)]
#![feature(auto_traits)]
#![feature(cfg_match)]
#![feature(core_intrinsics)]
#![feature(dropck_eyepatch)]
#![feature(extend_one)]
#![feature(file_buffered)]
#![feature(macro_metavar_expr)]
#![feature(map_try_insert)]
#![feature(min_specialization)]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(ptr_alignment_type)]
#![feature(rustc_attrs)]
#![feature(rustdoc_internals)]
#![feature(test)]
#![feature(thread_id_value)]
#![feature(type_alias_impl_trait)]
#![feature(unwrap_infallible)]

use std::fmt;

pub use atomic_ref::AtomicRef;
pub use ena::{snapshot_vec, undo_log, unify};

mod atomic_ref;
pub mod frozen;
pub mod stable_hasher;

#[inline(never)]
#[cold]
pub fn outline<F: FnOnce() -> R, R>(f: F) -> R {
    f()
}

pub fn defer<F: FnOnce()>(f: F) -> OnDrop<F> {
    OnDrop(Some(f))
}

pub struct OnDrop<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> OnDrop<F> {
    #[inline]
    pub fn disable(mut self) {
        self.0.take();
    }
}

impl<F: FnOnce()> Drop for OnDrop<F> {
    #[inline]
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

pub struct FatalErrorMarker;

pub fn make_display(f: impl Fn(&mut fmt::Formatter<'_>) -> fmt::Result) -> impl fmt::Display {
    struct Printer<F> {
        f: F,
    }
    impl<F> fmt::Display for Printer<F>
    where
        F: Fn(&mut fmt::Formatter<'_>) -> fmt::Result,
    {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            (self.f)(fmt)
        }
    }

    Printer { f }
}

#[doc(hidden)]
pub fn __noop_fix_for_windows_dllimport_issue() {}

#[macro_export]
macro_rules! external_bitflags_debug {
    ($Name:ident) => {
        impl ::std::fmt::Debug for $Name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::bitflags::parse::to_writer(self, f)
            }
        }
    };
}
