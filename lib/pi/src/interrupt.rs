use crate::common::IO_BASE;

use volatile::prelude::*;
use volatile::{ReadVolatile, Volatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

impl Interrupt {
    pub const MAX: usize = 8;

    pub fn iter() -> impl Iterator<Item = Interrupt> {
        use Interrupt::*;
        [Timer1, Timer3, Usb, Gpio0, Gpio1, Gpio2, Gpio3, Uart]
            .iter()
            .map(|int| *int)
    }
}

impl From<usize> for Interrupt {
    fn from(irq: usize) -> Interrupt {
        use Interrupt::*;
        match irq {
            1 => Timer1,
            3 => Timer3,
            9 => Usb,
            49 => Gpio0,
            50 => Gpio1,
            51 => Gpio2,
            52 => Gpio3,
            57 => Uart,
            _ => panic!("Unknown irq: {}", irq),
        }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    pending_basic: Volatile<u32>,
    pending1: Volatile<u32>,
    pending2: Volatile<u32>,
    fiq_ctrl: Volatile<u32>,
    enable_irqs1: Volatile<u32>,
    enable_irqs2: Volatile<u32>,
    enable_irqs_basic: Volatile<u32>,
    disable_irqs1: Volatile<u32>,
    disable_irqs2: Volatile<u32>,
    disable_irqs_basic: Volatile<u32>,
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers,
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
        let int = int as usize;
        if int <= 31 {
            self.registers.enable_irqs1.or_mask(1 << int);
        } else if int <= 63 {
            self.registers.enable_irqs2.or_mask(1 << (int - 31));
        }
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
        let int = int as usize;
        if int <= 31 {
            self.registers.disable_irqs1.or_mask(1 << int);
        } else if int <= 63 {
            self.registers.disable_irqs2.or_mask(1 << (int - 31));
        }
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
        let int = int as usize;
        if int <= 31 {
            self.registers.pending1.has_mask(1 << int)
        } else if int <= 63 {
            self.registers.pending2.has_mask(1 << (int - 31))
        } else {
            false
        }
    }

    /// Enables the interrupt as FIQ interrupt
    pub fn enable_fiq(&mut self, int: Interrupt) {
        // Lab 5 2.B
        unimplemented!("enable_fiq")
    }
}
