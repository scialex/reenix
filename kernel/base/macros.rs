
#![macro_escape]

#[macro_export]
macro_rules! not_yet_implemented(
    ($name:expr) => (panic!(concat!(stringify!($name), " is not yet implemented")));
    ($name:expr, $msg:expr) => (not_yet_implemented!($name, "{}", $msg));
    ($name:expr, $msg:expr, $($v:expr),*) => (panic!(concat!(stringify!($name), " is not yet implemented ", $msg, $($v),*)))
)
