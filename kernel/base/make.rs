// TODO Copyright Header
//! Stuff for creating values of certain types.

use core::prelude::*;
/// A trait for a type that has a initializer that takes a single value of type K.
pub trait Make<A> {
    /// Create a value using the type K as an initializer.
    fn make(a: A) -> Self;
}

impl<A> Make<A> for A { fn make(a: A) -> A { a } }
impl<'a, 'b: 'a, A> Make<&'a A> for A where A: Clone + 'b { fn make(a: &'a A) -> A { a.clone() } }

/// A trait for creating a value using a reference to another one.
///
/// The generated value might outlive the reference used to create it and should not hold a
/// reference to it
pub trait RefMake<'a, A: 'a> {
    /// Make this value from a reference to another type, which might not live as long as the
    /// generated value.
    fn make_from<'b>(v: &'b A) -> Self;
}

impl<A> RefMake<'static, A> for A where A: Clone + 'static { fn make_from<'b>(v: &'b A) -> A { v.clone() }}

/// A trait where one attempts to make a value but it can fail.
pub trait TryMake<A, E> {
    /// Make a value from the given input or fail with error E.
    fn try_make(a: A) -> Result<Self, E>;
}

impl<A, R> TryMake<A, ()> for R where R: Make<A> { fn try_make(a: A) -> Result<R, ()> { Ok(Make::make(a)) } }
