// TODO Copyright Header

#![allow(missing_doc)]
#![feature(macro_rules)]

macro_rules! set_type (
    ($func:id is $($v:expr),*) => (
        #[inline]
        pub fn $func(c: u8) -> bool {
            match (c) {
                $(
                    $v => true
                 )*
                    _ => false
            }
        }
    )
)

set_type!(is_blank is ' ', '\t');
set_type!(is_space is ' ', '\t', '\f', '\n', '\r', '\v');
set_type!(is_upper is 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
                      'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
                      'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z');
set_type!(is_lowwer is 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i',
                       'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
                       's', 't', 'u', 'v', 'w', 'x', 'y', 'z');

#[inline] pub fn is_alpha(c: u8) -> bool { is_lower(c) || is_upper(c) }
set_type!(is_digit is '1', '2', '3', '4', '5', '6', '7', '8', '9', '0');

#[inline] pub fn is_alphanum(c: u8) -> bool { is_alpha(c) || is_digit(c) }

set_type!(is_punc is ',', '.', '/', '<', '>', '?', ';', ':', '"',
                     '[', ']', '{', '}', '`', '-', '=', '+', '*',
                     '~', '!', '@', '#', '$', '%', '^', '&', '(',
                     ')', '_', '\'');

set_type!(is_xdigit is '1','2','3','4','5','6','7','8','9','0',
                       'a','b','c','d','e','f','A','B','C','D',
                       'E','F');
#[inline] pub fn is_ascii(c: u8) -> bool { c < 0x7f }

#[inline] pub fn is_graph(c: u8) -> bool { is_punc(c) || is_alphanum(c) }
#[inline] pub fn is_print(c: u8) -> bool { c == ' ' || is_graph(c) }
#[inline] pub fn is_cntrl(c: u8) -> bool { is_ascii(c) && !is_print(c) }
