use core::fmt;
use shim::const_assert_size;

#[repr(C)]
#[derive(Default, Copy, Clone, Debug)]
pub struct TrapFrame {
    pub link_addr: u64,
    pub pstate: u64,
    pub sp: u64,
    pub tpidr: u64,
    pub ttbr0: u64,
    pub ttbr1: u64,
    simd_reg: [u128; 32],
    pub gen_reg: [u64; 32],
}

const_assert_size!(TrapFrame, 816);

impl TrapFrame {
    pub fn increment_link_addr(&mut self, increment: u64) {
        self.link_addr += increment;
    }
}
