#![allow(internal_features)]
#![allow(rustc::internal)]
#![cfg_attr(test, feature(test))]
#![feature(core_intrinsics)]
#![feature(min_specialization)]
#![feature(never_type)]
#![feature(rustdoc_internals)]

pub use self::serialize::{Decodable, Decoder, Encodable, Encoder};

mod serialize;

pub mod int_overflow;
pub mod leb128;
pub mod opaque;
