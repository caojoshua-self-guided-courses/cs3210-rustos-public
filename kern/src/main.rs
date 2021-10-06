#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

pub mod console;
pub mod mutex;
pub mod shell;

use console::kprintln;

// FIXME: You need to add dependencies here to
// test your drivers (Phase 2). Add them as needed.
use pi::timer;
use core::time::Duration;
const GPIO_BASE: usize = 0x3F000000 + 0x200000;

const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

unsafe fn kmain() -> ! {
    // FIXME: Start the shell.
    GPIO_FSEL1.write_volatile(1 << 18);
    loop {
        GPIO_SET0.write_volatile(1 << 16);
        timer::spin_sleep(Duration::from_micros(500000));
        GPIO_CLR0.write_volatile(1 << 16);
        timer::spin_sleep(Duration::from_micros(500000));
    }
}
