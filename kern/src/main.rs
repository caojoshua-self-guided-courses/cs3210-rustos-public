#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
#![feature(ptr_internals)]
#![feature(raw_vec_internals)]
#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

#[cfg(not(test))]
mod init;

extern crate alloc;

pub mod allocator;
pub mod console;
pub mod fs;
pub mod mutex;
pub mod shell;
pub mod param;
pub mod process;
pub mod traps;
pub mod vm;

use console::kprintln;

use allocator::Allocator;
use fs::FileSystem;
use process::GlobalScheduler;
use traps::irq::Irq;
use vm::VMManager;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();
pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();
pub static VMM: VMManager = VMManager::uninitialized();
pub static IRQ: Irq = Irq::uninitialized();

fn kmain() -> ! {
    pi::timer::spin_sleep(core::time::Duration::from_secs(2));
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }

    unsafe {
        // Testing various exceptions
        aarch64::brk!(2);
        // aarch64::svc!(3);
        // kprintln!("{}", *(0x00000000 as *const u64))
        // kprintln!("{}", *(0xFFFFFFFF as *const u64))
        // kprintln!("{}", *(0xFFFFFFFFFFFFFFFF as *const u64))
    }

    kprintln!("Welcome to cs3210!");
    shell::shell("> ");

    loop {
        aarch64::nop()
    }
}
