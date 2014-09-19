// TODO Copyright Header

//! The reenix allocation support.
///
/// This provides support for malloc and free.
///
/// At the moment it simply goes down to the 'C' slab allocators but this might change in the
/// future if I ever get around to actually implementing a rust allocator.

pub static SLAB_REDZONE : u32 = 0xdeadbeef;


