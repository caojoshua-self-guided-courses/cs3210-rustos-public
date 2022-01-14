use alloc::boxed::Box;
use shim::io::Read;
use shim::path::Path;

use fat32::traits::{Entry, File, FileSystem};

use crate::FILESYSTEM;
use crate::param::*;
use crate::process::{Stack, State};
use crate::traps::TrapFrame;
use crate::vm::*;
use kernel_api::{OsError, OsResult};

/// Type alias for the type of a process ID.
pub type Id = u64;

/// A structure that represents the complete state of a process.
#[derive(Debug)]
pub struct Process {
    /// The saved trap frame of a process.
    pub context: Box<TrapFrame>,
    /// The memory allocation used for the process's stack.
    pub stack: Stack,
    /// The page table describing the Virtual Memory of the process
    pub vmap: Box<UserPageTable>,
    /// The scheduling state of the process.
    pub state: State,
}

impl Process {
    /// Creates a new process with a zeroed `TrapFrame` (the default), a zeroed
    /// stack of the default size, and a state of `Ready`.
    ///
    /// If enough memory could not be allocated to start the process, returns
    /// `None`. Otherwise returns `Some` of the new `Process`.
    pub fn new() -> OsResult<Process> {
        let stack = match Stack::new() {
            Some(stack) => stack,
            None => return Err(OsError::NoMemory),
        };

        Ok(Process {
            context: Box::new(TrapFrame::default()),
            stack,
            vmap: Box::new(UserPageTable::new()),
            state: State::Ready,
        })
    }

    /// Load a program stored in the given path by calling `do_load()` method.
    /// Set trapframe `context` corresponding to the its page table.
    /// `sp` - the address of stack top
    /// `elr` - the address of image base.
    /// `ttbr0` - the base address of kernel page table
    /// `ttbr1` - the base address of user page table
    /// `spsr` - `F`, `A`, `D` bit should be set.
    ///
    /// Returns Os Error if do_load fails.
    pub fn load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        use crate::VMM;

        let mut p = Process::do_load(pn)?;

        p.context.sp = Process::get_stack_top().as_u64();
        p.context.link_addr = USER_IMG_BASE as u64;
        p.context.ttbr0 = VMM.get_baddr().as_u64();
        p.context.ttbr1 = p.vmap.get_baddr().as_u64();

        // Set the exception level to 0 (second/third bit).
        p.context.pstate &= !0b1100;

        // Unmask IRQ and make `F`, `A`, and `D`.
        p.context.pstate |= 0b1101 << 6;
        p.context.pstate &= !(0b01 << 7);

        Ok(p)
    }

    /// Creates a process and open a file with given path.
    /// Allocates one page for stack with read/write permission, and N pages with read/write/execute
    /// permission to load file's contents.
    fn do_load<P: AsRef<Path>>(pn: P) -> OsResult<Process> {
        let mut p = Process::new()?;

        let mut file = match FILESYSTEM.open(pn)?.into_file() {
            Some(file) => file,
            None => return Err(OsError::ExpectedFileFoundDir),
        };

        p.vmap.alloc(VirtualAddr::from(Process::get_stack_base()), PagePerm::RW);

        let size = file.size() as usize;
        let mut addr = USER_IMG_BASE;
        let end_addr = addr + size;

        while addr < end_addr {
            let bytes = p.vmap.alloc(VirtualAddr::from(addr), PagePerm::RWX);
            file.read(bytes)?;
            addr += PAGE_SIZE;
        }

        Ok(p)
    }

    /// Returns the highest `VirtualAddr` that is supported by this system.
    pub fn get_max_va() -> VirtualAddr {
        VirtualAddr::from(USER_MAX_VM_SIZE - 1) + Process::get_image_base()
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// memory space.
    pub fn get_image_base() -> VirtualAddr {
        VirtualAddr::from(USER_IMG_BASE)
    }

    /// Returns the `VirtualAddr` represents the base address of the user
    /// process's stack.
    pub fn get_stack_base() -> VirtualAddr {
        // Set the stack base to be the address of the last page. Make sure the result is aligned
        // by the page_size, even though it should already be aligned by hard coded values.
        Process::get_max_va() - VirtualAddr::from(PAGE_SIZE) + VirtualAddr::from(1) &
            VirtualAddr::from(!(PAGE_SIZE - 1))
    }

    /// Returns the `VirtualAddr` represents the top of the user process's
    /// stack.
    pub fn get_stack_top() -> VirtualAddr {
        // Not sure about this one. If the stack is at the top of the VA space, then it would make
        // sense the stack top would also be the max VA>.
        // Strip the last 4 bits to make sure the result is 16 byte aligned.
        Process::get_max_va() & VirtualAddr::from(!(Stack::ALIGN - 1))
    }

    /// Returns `true` if this process is ready to be scheduled.
    ///
    /// This functions returns `true` only if one of the following holds:
    ///
    ///   * The state is currently `Ready`.
    ///
    ///   * An event being waited for has arrived.
    ///
    ///     If the process is currently waiting, the corresponding event
    ///     function is polled to determine if the event being waiting for has
    ///     occured. If it has, the state is switched to `Ready` and this
    ///     function returns `true`.
    ///
    /// Returns `false` in all other cases.
    pub fn is_ready(&mut self) -> bool {
        if let State::Waiting(event_poll_fn) = &mut self.state {
            // Need to use mem::replace because we can't use original event_poll_fn, because it
            // borrows self, and we are already borrowing state from self.
            let mut event_poll_fn_copy = core::mem::replace(event_poll_fn, Box::new(|_| false));

            if event_poll_fn_copy(self) {
                self.state = State::Ready;
            }

            // Reset the polling function. Can't reuse event_poll_fn because it is borrowed from
            // state, and the copy also borrows self.
            if let State::Waiting(event_poll_fn) = &mut self.state {
                core::mem::replace(event_poll_fn, event_poll_fn_copy);
            }
        }

        match self.state {
            State::Ready => true,
            _ => false,
        }
    }
}
