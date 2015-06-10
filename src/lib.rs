#![feature(box_syntax, core, alloc)]

mod raw;
pub mod xorlist;

#[doc(inline)]
pub use xorlist::XorList;
