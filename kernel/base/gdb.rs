// TODO Copyright Header

#![macro_escape]

#[macro_export]
macro_rules! gdb_define_hook(
    ($name:ident, $($a:ty),*) => {
        mod gdb_hooks {
            #[allow(unused)]
            #[allow(missing_doc)]
            #[inline(never)]
            #[export_name=concat!("__py_hook_",stringify!(name))]
            pub fn $name (_: $($a),*) {}
        }
    }
)
