// TODO Copyright Header

//! A FromStr copy from std.

use core::num::{Num, NumCast, cast, zero, CheckedMul, CheckedAdd};
use core::option::{Option,None,Some};
use core::str::StrSlice;
use core::char::Char;
use core::collections::Collection;

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

fn from_str_common<A: CheckedAdd + CheckedMul + Num + NumCast>(s: &str) -> Option<A> {
    match s.len() {
        0 => { return None; },
        1 => { return cast::<uint, A>(early_return!(s.char_at(0).to_digit(10))); },
        _ => {
            let is_negative = s.char_at(0) == '-';
            let str_start = if is_negative { 1 } else { 0 };
            let tot_len = s.len();
            let (radix, num_start) = if s.char_at(str_start) == '0' {
                if str_start + 1 == tot_len {
                    return Some(zero());
                } else {
                    match s.char_at(str_start + 1) {
                        'X' => (16, str_start + 1),
                        'x' => (16, str_start + 1),
                        'B' => (2,  str_start + 1),
                        'b' => (2,  str_start + 1),
                        'D' => (10, str_start + 1),
                        'd' => (10, str_start + 1),
                        'O' => (8,  str_start + 1),
                        'o' => (8,  str_start + 1),
                        _ => if s.char_at(str_start + 1).is_digit_radix(10) { (10, str_start) } else { return None; }
                    }
                }
            } else { (10, str_start) };
            let mut val: A = zero();
            for c in s.slice_from(num_start).chars() {
                val = early_return!(val.checked_mul(&early_return!(cast::<uint,A>(radix))));
                val = early_return!(val.checked_add(&early_return!(cast::<uint,A>(early_return!(c.to_digit(radix))))));
            }
            if is_negative {
                return Some(-val);
            } else {
                return Some(val);
            }
        }
    }
}

macro_rules! make_from_str(
    ($t:ty) => (
        impl FromStr for $t {
            #[inline] fn from_str(s: &str) -> Option<$t> { from_str_common::<$t>(s) }
        }
    )
)

make_from_str!(i8)
make_from_str!(i16)
make_from_str!(i32)
make_from_str!(i64)
make_from_str!(int)
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

