#![feature(alloc_error_handler)]
#![feature(const_fn)]
#![feature(decl_macro)]
#![feature(asm)]
#![feature(global_asm)]
#![feature(optin_builtin_traits)]
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

use console::kprintln;

use allocator::Allocator;
use fs::FileSystem;

use allocator::memory_map;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {
    unsafe {
        // ALLOCATOR.initialize();
        // FILESYSTEM.initialize();
    }

    for _ in 0 .. 3 {
        let (start, end) = memory_map().expect("failed to find memory map");
        kprintln!("start, end : {}, {}", start, end);
        pi::timer::spin_sleep(core::time::Duration::from_secs(1));
    }

    kprintln!("Welcome to cs3210!");
    shell::shell("> ");
}
