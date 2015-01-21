// TODO Copyright Header
//

/// A bitmask creation macro.
///
/// It is used by specifying the name you wish to give the particular bit and the number of bits to
/// shift for that bit, starting at zero. One can optionally put a `default` name which is the name
/// of the value if it has no bits set. It is pretty printed to be a `|` of all the names whose
/// bits are set.
#[macro_export]
macro_rules! bitmask_create {
    ($(#[$base:meta])* flags $name:ident : $t:ty
     {  #[$hm:meta] default $d:ident, $(#[$m:meta] $f:ident = $v:expr),+ }) => {
        #[derive(Default, PartialEq, Eq, Copy, Clone)]
        $(#[$base])*
        pub struct $name($t);
        $(#[$m] pub const $f : $name = $name(0x1 << $v);)*
        #[$hm] pub const $d : $name = $name(0);
        bitmask_create!(inner_flags $name { $($f,)* $d });
    };
    ($(#[$base:meta])* flags $name:ident : $t:ty
     { $(#[$m:meta] $f:ident = $v:expr),+ }) => {
        #[derive(Default, PartialEq, Eq, Copy, Clone)]
        $(#[$base])*
        pub struct $name($t);
        $(#[$m] pub const $f : $name = $name(0x1 << $v);)*
        bitmask_create!(inner_flags $name { $($f),* });
    };
    ($(#[$base:meta])* flags $name:ident : $t:ty
     {  default $d:ident, $($f:ident = $v:expr),+ }) => {
        #[derive(Default, PartialEq, Eq, Copy, Clone)]
        $(#[$base])*
        pub struct $name($t);
        $(pub const $f : $name = $name(0x1 << $v);)*
        pub const $d : $name = $name(0);
        bitmask_create!(inner_flags $name { $($f,)* $d });
    };
    ($(#[$base:meta])* flags $name:ident : $t:ty
     { $($f:ident = $v:expr),+ }) => {
        #[derive(Default, PartialEq, Eq, Copy, Clone)]
        $(#[$base])*
        pub struct $name($t);
        $(pub const $f : $name = $name(0x1 << $v);)*
        bitmask_create!(inner_flags $name { $($f),* });
    };
    (inner_flags $name:ident { $($f:ident),+ }) => {
        impl ::std::fmt::Show for $name {
            fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                (self as &::std::fmt::String).fmt(fmt)
            }
        }
        impl ::std::fmt::String for $name {
            fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                try!(write!(fmt, "{}[",stringify!($name)));
                let mut started = false;
                $(
                if *self == $f || ($f != $name(0) && *self & $f != $name(0)) {
                    if started { try!(write!(fmt, "|")); } else { started = true; }
                    try!(write!(fmt, stringify!($f)));
                }
                )+
                if !started { try!(write!(fmt, "0")); }
                write!(fmt, "]")
            }
        }
        impl ::std::ops::BitXor for $name {
            type Output = $name;
            #[inline] fn bitxor(self, r: $name) -> $name {
                let $name(lhs) = self;
                let $name(rhs) = r;
                $name(lhs ^ rhs)
            }
        }
        impl ::std::ops::BitOr for $name {
            type Output = $name;
            #[inline] fn bitor(self, r: $name) -> $name {
                let $name(lhs) = self;
                let $name(rhs) = r;
                $name(lhs | rhs)
            }
        }
        impl ::std::ops::BitAnd for $name {
            type Output = $name;
            #[inline] fn bitand(self, r: $name) -> $name {
                let $name(lhs) = self;
                let $name(rhs) = r;
                $name(lhs & rhs)
            }
        }
        impl ::std::ops::Add for $name {
            type Output = $name;
            #[inline] fn add(self, r: $name) -> $name { self | r }
        }
        impl ::std::ops::Sub for $name {
            type Output = $name;
            #[inline] fn sub(self, r: $name) -> $name {
                self & (!r)
            }
        }
        impl ::std::ops::Not for $name {
            type Output = $name;
            #[inline] fn not(self) -> $name {
                let $name(val) = self;
                $name(!val)
            }
        }
    };
}
