use core::time::Duration;

use volatile::prelude::*;
use volatile::{Reserved, Volatile};

use aarch64::regs::*;
use shim::const_assert_size;

const INT_BASE: usize = 0x40000000;

/// Core interrupt sources (QA7: 4.10)
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LocalInterrupt {
    // Lab 5 1.C
    CNTPSIRQ = 0,
    CNTPNSIRQ = 1,
    CNTHPIRQ = 2,
    CNTVIRQ = 3,
    MAILBOX0 = 4,
    MAILBOX1 = 5,
    MAILBOX2 = 6,
    MAILBOX3 = 7,
    GPU = 8,
    PMU = 9,
    AxiOutstanding = 10,
    LocalTimer = 11,
    Unknown = 12,
}

impl LocalInterrupt {
    pub const MAX: usize = 12;

    pub fn iter() -> impl Iterator<Item = LocalInterrupt> {
        (0..LocalInterrupt::MAX).map(|n| LocalInterrupt::from(n))
    }
}

impl From<usize> for LocalInterrupt {
    fn from(irq: usize) -> LocalInterrupt {
        // Lab 5 1.C
        use LocalInterrupt::*;
        match irq {
            0 => CNTPSIRQ,
            1 => CNTPNSIRQ,
            2 => CNTHPIRQ,
            3 => CNTVIRQ,
            4 => MAILBOX0,
            5 => MAILBOX1,
            6 => MAILBOX2,
            7 => MAILBOX3,
            8 => GPU,
            9 => PMU,
            10 => AxiOutstanding,
            11 => LocalTimer,
            _ => Unknown,
        }
    }
}

/// BCM2837 Local Peripheral Registers (QA7: Chapter 4)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    control: Volatile<u32>,
    __r0: Reserved<u32>,
    core_timer_prescaler: Volatile<u32>,
    gpu_int: Volatile<u32>,
    perf_monitor_int_set: Volatile<u32>,
    perf_monitor_int_clear: Volatile<u32>,
    __r1: Reserved<u32>,
    core_timer_access_ls: Volatile<u32>,
    core_timer_access_ms: Volatile<u32>,
    local_interrupt: Volatile<u32>,
    __r2: Reserved<u32>,
    axi_outstanding_counters: Volatile<u32>,
    axi_outstanding_irq: Volatile<u32>,
    local_timer_ctrl: Volatile<u32>,
    local_timer_write: Volatile<u32>,
    __r3: Reserved<u32>,
    core_timer_int: [Volatile<u32>; 4],
    core_mailbox_int: [Volatile<u32>; 4],
    core_irq_src: [Volatile<u32>; 4],
    core_fiq_src: [Volatile<u32>; 4],
}

const_assert_size!(Registers, 128);

pub struct LocalController {
    core: usize,
    registers: &'static mut Registers,
}

impl LocalController {
    /// Returns a new handle to the interrupt controller.
    pub fn new(core: usize) -> LocalController {
        LocalController {
            core: core,
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    pub fn enable_local_timer(&mut self) {
        // Lab 5 1.C
        unsafe {
            CNTP_CTL_EL0.set(CNTP_CTL_EL0.get() | CNTP_CTL_EL0::ENABLE);
            CNTP_CTL_EL0.set(CNTP_CTL_EL0.get() & !CNTP_CTL_EL0::IMASK);
        };
        // CNTPNS is bit 1
        self.registers.core_timer_int[self.core].or_mask(0b10);
    }

    pub fn is_pending(&self, int: LocalInterrupt) -> bool {
        // Lab 5 1.C
        let reg = &self.registers.core_irq_src[self.core];
        reg.has_mask(1 << int as usize)
    }

    pub fn tick_in(&mut self, t: Duration) {
        // Lab 5 1.C
        // See timer: 3.1 to 3.3
        //
        // I honestly just copied this from other people who did the class. The instructions don't
        // really provide anything, and I have no idea where the constant comes from. Maybe this
        // was discussed in the class, but the information was never propagated to online
        // resources.
        let freq = unsafe { CNTFRQ_EL0.get() };
        let ticks = freq as u128 * t.as_micros() / 1000000;
        unsafe { CNTP_TVAL_EL0.set(ticks as u64) };
    }
}

pub fn local_tick_in(core: usize, t: Duration) {
    LocalController::new(core).tick_in(t);
}
