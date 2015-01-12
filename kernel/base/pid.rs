
use core::prelude::*;
use core::ops::Add;

/// The ProcId struct. Needs to be here for dependency reasons.

#[cfg(not(SMALL_PID))] pub type PidInner = u32;
#[cfg(SMALL_PID)]      pub type PidInner = u8;

#[derive(Eq, PartialEq, Show, PartialOrd, Ord, Clone, Copy)] #[repr(C)]
pub struct ProcId(pub PidInner);

impl Add<uint> for ProcId {
    type Output = ProcId;
    fn add(self, rhs: uint) -> ProcId {
        let ProcId(v) = self;
        ProcId(v + (rhs as PidInner))
    }
}
