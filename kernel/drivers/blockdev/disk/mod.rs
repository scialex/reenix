//! The REENIX disk stuff

pub fn init_stage1() {
    ata::init_stage1();
    dma::init_stage1();
}
pub fn init_stage2() {
    ata::init_stage2();
    dma::init_stage2();
}

mod ata;
mod dma;
