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

unsafe fn kmain() -> ! {
    // FIXME: Start the shell.
    let mut output = [
        pi::gpio::Gpio::new(5).into_output(),
        pi::gpio::Gpio::new(6).into_output(),
        pi::gpio::Gpio::new(13).into_output(),
        pi::gpio::Gpio::new(16).into_output(),
        pi::gpio::Gpio::new(19).into_output(),
        pi::gpio::Gpio::new(26).into_output(),
    ];
    let num_outputs = output.len();

    output[0].set();
    output[1].set();
    pi::timer::spin_sleep(core::time::Duration::from_micros(200000));

    let mut a = 0;
    let mut b = 1;
    loop {
        output[a].clear();
        a = a + 1;
        b = b + 1;
        if a == num_outputs {
            a = 0;
        } else if b == num_outputs {
            b = 0;
        }
        output[b].set();

        pi::timer::spin_sleep(core::time::Duration::from_micros(200000));
    }
}
