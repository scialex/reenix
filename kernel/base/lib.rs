// TODO Copyright Header

//! # The Reenix base util stuff.

#![crate_name="base"]
#![crate_type="rlib"]
#![no_std]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(asm, macro_rules, globs, concat_idents,lang_items, trace_macros, phase)]


#[phase(plugin)] extern crate enabled;
#[phase(plugin, link)] extern crate core;
extern crate libc;

//pub use errno::*;

// NOTE Needs to go first so everything else can get the macro's defined in it.
mod bitflags;
mod macros;

pub mod devices;

pub mod gdb;

pub mod errno;

pub mod io;
pub mod debug;
pub mod kernel;

pub mod make {
    /// A trait for a type that has a initializer that takes a single value of type K.
    pub trait Make<A> {
        /// Create a value using the type K as an initializer.
        fn make(a: A) -> Self;
    }

    /// A trait for creating a value using a reference to another one.
    ///
    /// The generated value might outlive the reference used to create it and should not hold a
    /// reference to it
    pub trait RefMake<'a, A: 'a> {
        /// Make this value from a reference to another type, which might not live as long as the
        /// generated value.
        fn make_from<'b, 'a: 'b>(v: &'b A) -> Self;
    }
}

pub mod describe {
    use core::fmt;
    use core::prelude::*;
    pub trait Describeable {
        fn describe(&self, &mut fmt::Formatter) -> fmt::Result;
    }
    pub struct Describer<T: Describeable>(T);
    impl<T: Describeable> Describeable for Describer<T> {
        fn describe(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let &Describer(ref x) = self;
            try!(write!(f, "Describe("));
            try!(x.describe(f));
            write!(f, ")")
        }
    }
    impl<T: Describeable> fmt::Show for Describer<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let &Describer(ref x) = self;
            x.describe(f)
        }
    }
}

pub fn init_stage1() { debug::setup(); }
pub fn init_stage2() {}

// NOTE Needed for the #[deriving] stuff to work. Because that makes sense.
#[doc(hidden)]
mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
    pub use core::default;
    pub use core::clone;
}

// This lets us use the macro's exported from here locally.
#[doc(hidden)]
mod base {
    pub use super::*;
}
