use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use alloc::vec::Vec;
use shim::path::Path;

use pi::interrupt::{Controller, Interrupt};
use pi::timer::{current_time, tick_in};

use core::ffi::c_void;
use core::fmt;
use core::mem;
use core::time::Duration;

use aarch64::*;
use pi::local_interrupt::{local_tick_in, LocalController, LocalInterrupt};
use smoltcp::time::Instant;

use crate::GLOBAL_IRQ;
use crate::mutex::Mutex;
use crate::net::uspi::TKernelTimerHandle;
use crate::param::*;
use crate::percore::{get_preemptive_counter, is_mmu_ready, local_irq};
use crate::process::{Id, Process, State};
use crate::traps::irq::IrqHandlerRegistry;
use crate::traps::TrapFrame;
use crate::{ETHERNET, USB};

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Box<Scheduler>>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enters a critical region and execute the provided closure with a mutable
    /// reference to the inner scheduler.
    pub fn critical<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Scheduler) -> R,
    {
        let mut guard = self.0.lock();
        f(guard.as_mut().expect("scheduler uninitialized"))
    }

    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.critical(move |scheduler| scheduler.add(process))
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::schedule_out()` and `Scheduler::switch_to()`.
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Id {
        self.critical(|scheduler| scheduler.schedule_out(new_state, tf));
        self.switch_to(tf)
    }

    /// Loops until it finds the next process to schedule.
    /// Call `wfi()` in the loop when no process is ready.
    /// For more details, see the documentation on `Scheduler::switch_to()`.
    ///
    /// Returns the process's ID when a ready process is found.
    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                trace!(
                    "[core-{}] switch_to {:?}, pc: {:x}, lr: {:x}, x29: {:x}, x28: {:x}, x27: {:x}",
                    affinity(),
                    id,
                    tf.link_addr,
                    tf.gen_reg[30],
                    tf.gen_reg[29],
                    tf.gen_reg[28],
                    tf.gen_reg[27]
                );
                return id;
            }

            aarch64::wfi();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal
    /// conditions.
    pub fn start(&self) -> ! {
        // Start the first process and get the trap frame.
        let mut tf = TrapFrame::default();
        self.critical(|scheduler| scheduler.switch_to(&mut tf));

        if affinity() == 0 {
            self.initialize_global_timer_interrupt();
        }
        self.initialize_local_timer_interrupt();

        let stack_base = KERN_STACK_BASE - (KERN_STACK_SIZE * (affinity() + 0));
        info!("stack_base affinity {}: {:x}", affinity(), stack_base);
        let tf_addr = (stack_base - core::mem::size_of::<TrapFrame>()) as *mut TrapFrame;

        // if SP >= tf_addr + core::mem::size_of( {
        // }
        // tf.tpidr = 69;
        // for i in 0..20 {
        //     tf.simd_reg[i] = 69;
        // }
        // tf.gen_reg[0] = 69;
        // this overwrites our current frame and messes up our pointers.
        // unsafe { *tf_addr = tf };

        // if affinity() == 0 {
        //     info!("stack_base core {}: {:x}", affinity(), stack_base);
        //     info!("current SP core {}: {:x}", affinity(), SP.get());
        //     info!("tf_addr core {}: {:x}", affinity(), tf_addr as usize);
        //     info!("orig.. tf core {}: {:?}", affinity(), tf);
        //     unsafe {
        //         info!("copied tf core {}: {:?}", affinity(), *(tf_addr as *const TrapFrame));
        //     }
        // }
        //
        unsafe {
            let mut foo: u64;

            asm!("ldr x0, =_start
                 mov $0, x0"
                 : "=r"(foo)
                 :: "x0"
                 : "volatile");
            info!("foo: {:x}", foo);
        }

        unsafe {
            let mut foo: u64;
            asm!("mov x0, $1
                 mov $0, x0"
                 : "=r"(foo)
                 : "r"(stack_base)
                 : "x0"
                 : "volatile");

            info!("read-write test: {:x}", foo);
        }

        unsafe {
            // SP.set(tf_addr as usize);
            // SP.set(&tf as *const TrapFrame as usize);
            // asm!("bl context_restore" :::: "volatile");
            asm!("mov sp, $0
                 bl context_restore
                 mov sp, $1
                 // TODO: try compute stack_base right here!
                 // Make sure to save the registers being used.
                 // extern function might mess with register values.
                 eret"
                 :: "r"(&tf), "r"(stack_base)
                 :: "volatile");

            // This creates weird exceptions for some reason...but with our current implementation,
            // at least we are only wasting memory equal to the size of a frame.
            // SP.set(stack_base);

            // eret();
        }

        loop {}
    }

    /// # Lab 4
    /// Initializes the global timer interrupt with `pi::timer`. The timer
    /// should be configured in a way that `Timer1` interrupt fires every
    /// `TICK` duration, which is defined in `param.rs`.
    ///
    /// # Lab 5
    /// Registers a timer handler with `Usb::start_kernel_timer` which will
    /// invoke `poll_ethernet` after 1 second.
    pub fn initialize_global_timer_interrupt(&self) {
    }

    /// Initializes the per-core local timer interrupt with `pi::local_interrupt`.
    /// The timer should be configured in a way that `CntpnsIrq` interrupt fires
    /// every `TICK` duration, which is defined in `param.rs`.
    pub fn initialize_local_timer_interrupt(&self) {
        // Lab 5 2.C
        let mut controller = LocalController::new(affinity());
        controller.enable_local_timer();
        local_irq().register(LocalInterrupt::CNTPNSIRQ, Box::new(timer1_handler));
        controller.tick_in(TICK);
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler.
    pub unsafe fn initialize(&self) {
        // Initialize the scheduler.
        *self.0.lock() = Some(Scheduler::new());

        // Add initial userspace processes.
        for _ in 0..3 {
            self.add(Process::load(Path::new("/sleep")).unwrap());
            self.add(Process::load(Path::new("/fib")).unwrap());
        }
    }
    // The following method may be useful for testing Lab 4 Phase 3:
    //
    // * A method to load a extern function to the user process's page table.
    //
    // pub fn test_phase_3(&self, proc: &mut Process){
    //     use crate::vm::{VirtualAddr, PagePerm};
    //
    //     let mut page = proc.vmap.alloc(
    //         VirtualAddr::from(USER_IMG_BASE as u64), PagePerm::RWX);
    //
    //     let text = unsafe {
    //         core::slice::from_raw_parts(test_user_process as *const u8, 24)
    //     };
    //
    //     page[0..24].copy_from_slice(text);
    // }
}

/// Poll the ethernet driver and re-register a timer handler using
/// `Usb::start_kernel_timer`.
extern "C" fn poll_ethernet(_: TKernelTimerHandle, _: *mut c_void, _: *mut c_void) {
    // Lab 5 2.B
    unimplemented!("poll_ethernet")
}

/// Internal scheduler struct which is not thread-safe.
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Box<Scheduler> {
        Box::new(Scheduler {
            processes: VecDeque::new(),
            last_id: Some(0),
        })
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        let id = match self.last_id {
            Some(last_id) => last_id,
            None => 0,
        };

        process.context.tpidr = id;
        self.processes.push_back(process);
        self.last_id = Some(id + 1);

        Some(id)
    }

    /// Finds the currently running process, sets the current process's state
    /// to `new_state`, prepares the context switch on `tf` by saving `tf`
    /// into the current process, and push the current process back to the
    /// end of `processes` queue.
    ///
    /// If the `processes` queue is empty or there is no current process,
    /// returns `false`. Otherwise, returns `true`.
    fn schedule_out(&mut self, new_state: State, tf: &mut TrapFrame) -> bool {
        // Get the current running process on this processor core by matching the process id.
        for i in 0..self.processes.len() {
            let process = &mut self.processes[i];
            if process.context.tpidr == tf.tpidr {
                *process.context = *tf;
                process.state = new_state;
                let process = self.processes.remove(i).unwrap();
                self.processes.push_back(process);
                return true;
            }
        }

        // aarch64::sev();
        false
    }

    /// Finds the next process to switch to, brings the next process to the
    /// front of the `processes` queue, changes the next process's state to
    /// `Running`, and performs context switch by restoring the next process`s
    /// trap frame into `tf`.
    ///
    /// If there is no process to switch to, returns `None`. Otherwise, returns
    /// `Some` of the next process`s process ID.
    fn switch_to(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        let mut next_process = None;
        for process in &mut self.processes {
            if process.is_ready() {
                next_process = Some(process);
                break;
            }
        }

        let next_process = match next_process {
            Some(process) => process,
            None => return None,
        };

        next_process.state = State::Running;
        *tf = *next_process.context;
        Some(tf.tpidr)
    }

    /// Kills currently running process by scheduling out the current process
    /// as `Dead` state. Releases all process resources held by the process,
    /// removes the dead process from the queue, drops the dead process's
    /// instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        self.schedule_out(State::Dead, tf);
        match self.processes.pop_back() {
            Some(process) => Some(process.context.tpidr),
            None => None
        }
    }

    /// Releases all process resources held by the current process such as sockets.
    fn release_process_resources(&mut self, tf: &mut TrapFrame) {
        // Lab 5 2.C
        unimplemented!("release_process_resources")
    }

    /// Finds a process corresponding with tpidr saved in a trap frame.
    /// Panics if the search fails.
    pub fn find_process(&mut self, tf: &TrapFrame) -> &mut Process {
        for i in 0..self.processes.len() {
            if self.processes[i].context.tpidr == tf.tpidr {
                return &mut self.processes[i];
            }
        }
        panic!("Invalid TrapFrame");
    }
}

impl fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.processes.len();
        write!(f, "  [Scheduler] {} processes in the queue\n", len)?;
        for i in 0..len {
            write!(
                f,
                "    queue[{}]: proc({:3})-{:?} \n",
                i, self.processes[i].context.tpidr, self.processes[i].state
            )?;
        }
        Ok(())
    }
}

pub extern "C" fn  test_user_process() -> ! {
    loop {
        let ms = 10000;
        let error: u64;
        let elapsed_ms: u64;

        unsafe {
            asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
                 : "=r"(elapsed_ms), "=r"(error)
                 : "r"(ms)
                 : "x0", "x7"
                 : "volatile");
        }
    }
}
fn timer1_handler(tf: &mut TrapFrame) {
    local_tick_in(affinity(), TICK);
    crate::SCHEDULER.switch(State::Ready, tf);
}
