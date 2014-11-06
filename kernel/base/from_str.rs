// TODO Copyright Header

//! A FromStr copy from std.

use core::num::{Num, NumCast, zero, CheckedMul, CheckedAdd, Bounded, Signed};
use core::option::{Option,None,Some};
use core::str::StrSlice;
use core::slice::*;
use core::char::Char;

pub trait FromStr {
    fn from_str(s: &str) -> Option<Self>;
}

pub fn from_str<A: FromStr>(s: &str) -> Option<A> {
    FromStr::from_str(s)
}

macro_rules! early_return(
    ($v:expr) => ( match $v {
            Some(val) => val,
            None => { return None; }
        }
    )
)

const SPACES : &'static [char] = &[' ', '\t'];
fn from_str_signed<A: Signed + Bounded + CheckedAdd + CheckedMul + Num + NumCast>(s: &str) -> Option<A> {
    let n = s.trim_chars(SPACES);
    let (mul, v) = if n.len() > 0 && n.is_char_boundary(1) {
        match n.slice_to(1) {
            "-" => (true, n.slice_from(1)),
            _   => (false, n),
        }
    } else { return None; };
    from_str_common::<A>(v).map(
        |x| {
            if mul {
                if x == Bounded::max_value() { Bounded::min_value() } else { x * NumCast::from(-1i).expect("Signed should have -1") }
            } else { x }
        }
    )
}

fn from_str_common<A: Bounded + CheckedAdd + CheckedMul + Num + NumCast>(s: &str) -> Option<A> {
    let n = s.trim_chars(SPACES);
    let (radix,string) = if n.len() > 2 && n.is_char_boundary(1) && n.is_char_boundary(2) {
        match n.slice_to(2) {
            "0x" => (16, n.slice_from(2)),
            "0b" => (2,  n.slice_from(2)),
            "0o" => (8,  n.slice_from(2)),
            _ => (10, n),
        }
    } else { (10, n) };
    let mul : A = NumCast::from(radix).expect("We couldn't get our radix in the requested type");
    let mut val = zero::<A>();
    for c in string.chars() {
        if c == '_' || c == ',' {
            // We allow there to be seperators.
            continue;
        } else if c.is_digit_radix(radix) {
            if val != Bounded::max_value() {
                match NumCast::from(c.to_digit(radix).expect("is a radix digit")) {
                    Some(v) => {
                        let new_val = match val.checked_mul(&mul) {
                            Some(v) => v,
                            None    => Bounded::max_value(),
                        };
                        val = match new_val.checked_add(&v) {
                            Some(x) => x,
                            None    => Bounded::max_value(),
                        }
                    },
                    None => { return None; }
                }
            }
        } else {
            return None;
        }
    }
    Some(val)
}

macro_rules! make_signed_from_str(
    ($t:ty) => (
        impl FromStr for $t {
            #[inline] fn from_str(s: &str) -> Option<$t> { from_str_signed::<$t>(s) }
        }
    )
)

macro_rules! make_from_str(
    ($t:ty) => (
        impl FromStr for $t {
            #[inline] fn from_str(s: &str) -> Option<$t> { from_str_common::<$t>(s) }
        }
    )
)

make_signed_from_str!(i8)
make_signed_from_str!(i16)
make_signed_from_str!(i32)
make_signed_from_str!(i64)
make_signed_from_str!(int)
make_from_str!(u8)
make_from_str!(u16)
make_from_str!(u32)
make_from_str!(u64)
make_from_str!(uint)

impl FromStr for bool {
    fn from_str(s: &str) -> Option<bool> {
        match s {
            "true" => Some(true),
            "false" => Some(false),
            _ => None
        }
    }
}

