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
use console::kprint;

use allocator::Allocator;
use fs::FileSystem;
use fs::sd::Sd;
use fat32::traits::{ BlockDevice, Dir, Entry };

use alloc::vec::Vec;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {
    pi::timer::spin_sleep(core::time::Duration::from_secs(2));
    unsafe {
        ALLOCATOR.initialize();
        FILESYSTEM.initialize();
    }

    kprintln!("root entries:");
    let root = fat32::traits::FileSystem::open(&FILESYSTEM, shim::path::Path::new("/")).unwrap().into_dir().unwrap();
    let entries = root.entries().unwrap();
    kprintln!("done reading root entries");
    for entry in root.entries().unwrap() {
        kprintln!("entry: {}", fat32::traits::Entry::name(&entry));
    }

    kprintln!("Welcome to cs3210!");
    shell::shell("> ");
}
