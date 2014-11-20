// TODO Copyright Header
//
#![macro_escape]

/// A bitmask creation macro.
///
/// It is used by specifying the name you wish to give the particular bit and the number of bits to
/// shift for that bit, starting at zero. One can optionally put a `default` name which is the name
/// of the value if it has no bits set. It is pretty printed to be a `|` of all the names whose
/// bits are set.
#[macro_export]
macro_rules! bitmask_create {
    (flags $name:ident : $t:ty
     {  default $d:ident, $($f:ident = $v:expr),+ }) => {
        #[deriving(Default, PartialEq, Eq)]
        pub struct $name($t);
        $(pub const $f : $name = $name(0x1 << $v);)*
        pub const $d : $name = $name(0);
        bitmask_create!(inner_flags $name { $($f,)* $d })
    };
    (flags $name:ident : $t:ty
     { $($f:ident = $v:expr),+ }) => {
        #[deriving(Default, PartialEq, Eq)]
        pub struct $name($t);
        $(pub const $f : $name = $name(0x1 << $v);)*
        bitmask_create!(inner_flags $name { $($f),* })
    };
    (inner_flags $name:ident { $($f:ident),+ }) => {
        impl Show for $name {
            fn fmt(&self, fmt: &mut Formatter) -> Result {
                try!(fmt.write(stringify!($name).as_bytes()))
                try!(fmt.write("[".as_bytes()))
                let mut started = false;
                $(
                if *self == $f || ($f != $name(0) && *self & $f != $name(0)) {
                    if started { try!(fmt.write("|".as_bytes())) } else { started = true; }
                    try!(fmt.write(stringify!($f).as_bytes()))
                }
                )+
                if !started { try!(fmt.write("0".as_bytes())) }
                try!(fmt.write("]".as_bytes()))
                return Ok(());
            }
        }
        impl BitXor<$name,$name> for $name {
            #[inline] fn bitxor(&self, r: &$name) -> $name {
                let &$name(lhs) = self;
                let &$name(rhs) = r;
                $name(lhs ^ rhs)
            }
        }
        impl BitOr<$name,$name> for $name {
            #[inline] fn bitor(&self, r: &$name) -> $name {
                let &$name(lhs) = self;
                let &$name(rhs) = r;
                $name(lhs | rhs)
            }
        }
        impl BitAnd<$name,$name> for $name {
            #[inline] fn bitand(&self, r: &$name) -> $name {
                let &$name(lhs) = self;
                let &$name(rhs) = r;
                $name(lhs & rhs)
            }
        }
        impl Add<$name,$name> for $name {
            #[inline] fn add(&self, r: &$name) -> $name { self.bitor(r) }
        }
        impl Sub<$name,$name> for $name {
            #[inline] fn sub(&self, r: &$name) -> $name {
                *self & r.not()
            }
        }
        impl Not<$name> for $name {
            #[inline] fn not(&self) -> $name {
                let &$name(val) = self;
                $name(!val)
            }
        }
    };
}
