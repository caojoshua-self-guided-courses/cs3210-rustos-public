use alloc::boxed::Box;
use core::time::Duration;
use pi::timer::current_time;

use crate::console::{CONSOLE, kprint, kprintln};
use crate::process::{Process, State};
use crate::traps::TrapFrame;
use crate::SCHEDULER;
use kernel_api::*;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
///
/// Some interesting behavior is that if we call sys_sleep N millis after the last TICK, then the
/// next process that runs only executes for TICK - N seconds. In practice, ensuring that each
/// process runs for a minimum time before scheduling it out, which would be typical in a
/// round-robin scheduler. But I'm okay with this behavior cause it works.
pub fn sys_sleep(ms: u32, tf: &mut TrapFrame) {
    kprintln!("sleep for {} ms", ms);
    let start = current_time();
    let end = start + Duration::from_millis(ms.into());
    let boxed_fnmut = Box::new(move |p: &mut Process| -> bool {
        current_time() >= end
    });
    SCHEDULER.switch(State::Waiting(boxed_fnmut), tf);

    // Not really sure what is the true elapsed time. This is usually 0-1 ms no matter the passed
    // in ms, because the context switch only happens after returning from handle_exception. I
    // think the only way to get the true elapsed time would be in user space.
    tf.gen_reg[0] = (current_time() - start).as_millis() as u64;
}

/// Returns current time.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns two
/// parameter:
///  - current time as seconds
///  - fractional part of the current time, in nanoseconds.
pub fn sys_time(tf: &mut TrapFrame) {
    let current_time = current_time();
    let current_secs: u64 = current_time.as_secs();
    tf.gen_reg[0] = current_secs;
    tf.gen_reg[1] = (current_time - Duration::from_secs(current_secs)).as_nanos() as u64;
}

/// Kills current process.
///
/// This system call does not take paramer and does not return any value.
pub fn sys_exit(tf: &mut TrapFrame) {
    unimplemented!("sys_exit()");
}

/// Write to console.
///
/// This system call takes one parameter: a u8 character to print.
///
/// It only returns the usual status value.
pub fn sys_write(b: u8, tf: &mut TrapFrame) {
    kprint!("{}", b as char);
}

/// Returns current process's ID.
///
/// This system call does not take parameter.
///
/// In addition to the usual status value, this system call returns a
/// parameter: the current process's ID.
pub fn sys_getpid(tf: &mut TrapFrame) {
    unimplemented!("sys_getpid()");
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    match num as usize {
        NR_SLEEP => sys_sleep(tf.gen_reg[0] as u32, tf),
        NR_TIME => sys_time(tf),
        NR_WRITE => sys_write(tf.gen_reg[0] as u8, tf),
        _ => kprintln!("Unknown syscall ID {}", num),
    }
}
