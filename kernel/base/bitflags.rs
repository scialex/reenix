// TODO Copyright Header
//
#![macro_escape]

#[macro_export]
macro_rules! bitmask_create {
    (flags $name:ident : $t:ty
     { $($f:ident = $v:expr),+ }) => {
        #[deriving(PartialEq, Eq)]
        pub struct $name($t);
        $(pub static $f : $name = $name($v);)*
        impl Show for $name {
            fn fmt(&self, fmt: &mut Formatter) -> Result {
                try!(fmt.write(stringify!($name).as_bytes()))
                try!(fmt.write("[".as_bytes()))
                let mut started = false;
                $(
                if self & $f != $name(0) {
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
                $name(lhs | rhs)
            }
        }
        impl Add<$name,$name> for $name {
            #[inline] fn add(&self, r: &$name) -> $name {
                let &$name(lhs) = self;
                let &$name(rhs) = r;
                $name(lhs + rhs)
            }
        }
    }
}
