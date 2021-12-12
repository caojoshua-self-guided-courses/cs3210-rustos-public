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
use fat32::traits::BlockDevice;

use alloc::vec::Vec;

#[cfg_attr(not(test), global_allocator)]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();
pub static FILESYSTEM: FileSystem = FileSystem::uninitialized();

fn kmain() -> ! {
    unsafe {
        ALLOCATOR.initialize();
        // FILESYSTEM.initialize();
    }

    kprintln!("reading from sd card");
    // let mut vec: Vec<u8> = Vec::with_capacity(512);
    let mut vec = [0; 512];
    unsafe {
        let mut sd = Sd::new().unwrap();
        let foo = sd.read_sector(0, &mut vec);
        // let foo = sd.read_sector(0, vec.as_mut_slice());
    }
    for byte in vec.iter() {
        kprint!("{:x?} ", byte);
    }

    kprintln!("Welcome to cs3210!");
    shell::shell("> ");
}
