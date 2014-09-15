// TODO Copyright Header
//

#![feature(macro_rules)]
#![macro_escape]

macro_rules! bitmask_create {
    (flags $name:ident : $type:ty
     { $($f:ident = $v:expr),+ }) => {
        pub type $name = $type;
        $(pub static $f : $type = $v;)*
        impl Show for $name {
            use core::fmt::Result;
            use core::option::Option;
            use core::result::Err;
            use core::result::Ok;
            use core::fmt::Formatter;
            fn fmt(&self, fmt: &mut Formatter) -> Result {
                try!(fmt.write(Stringify!($name));)
                try!(fmt.write("[");)
                let mut started = false;
                $(
                if self & $f != 0 {
                    if started { try!(fmt.write("|");) } else { started = true; }
                    try!(fmt.write(Stringify!($f));)
                }
                )+
                if !started { try!(fmt.write("0");) }
                try!(fmt.write("]");)
                return OK(());
            }
        }
        impl $name {
            $(
            fn is_$(f)(e: $type) -> bool {
                e & $v != 0
            }
            )+
        }
    }
}
