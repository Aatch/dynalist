#![feature(box_syntax, core, alloc, unsafe_no_drop_flag)]
#![feature(optin_builtin_traits, filling_drop)]

extern crate core;

mod raw;
pub mod xorlist;
pub mod ilist;

#[doc(inline)]
pub use xorlist::XorList;

#[doc(inline)]
pub use ilist::IList;
