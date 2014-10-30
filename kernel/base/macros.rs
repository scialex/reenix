
#![macro_escape]

#[macro_export]
macro_rules! not_yet_implemented(
    ($name:expr) => (kpanic!(concat!(stringify!($name), " is not yet implemented")));
    ($name:expr, $msg:expr) => (not_yet_implemented!($name, "{}", $msg));
    ($name:expr, $msg:expr, $($v:expr),*) => (kpanic!(concat!(stringify!($name), " is not yet implemented ", $msg, $($v),*)))
)

#[macro_export]
macro_rules! describe(
    ($v:expr) => ({ use base::describe::Describer; Describer($v) })
)
