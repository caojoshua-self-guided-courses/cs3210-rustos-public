use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use shim::path::Path;

use pi::interrupt::{Controller, Interrupt};
use pi::timer::{current_time, tick_in};

use crate::mutex::Mutex;
use crate::param::TICK;
use crate::process::{Id, Process, State};
use crate::traps::TrapFrame;
use crate::IRQ;

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Enter a critical region and execute the provided closure with the
    /// internal scheduler.
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

    pub fn switch_to(&self, tf: &mut TrapFrame) -> Id {
        loop {
            let rtn = self.critical(|scheduler| scheduler.switch_to(tf));
            if let Some(id) = rtn {
                return id;
            }
            aarch64::wfe();
        }
    }

    /// Kills currently running process and returns that process's ID.
    /// For more details, see the documentaion on `Scheduler::kill()`.
    #[must_use]
    pub fn kill(&self, tf: &mut TrapFrame) -> Option<Id> {
        self.critical(|scheduler| scheduler.kill(tf))
    }

    /// Starts executing processes in user space using timer interrupt based
    /// preemptive scheduling. This method should not return under normal conditions.
    pub fn start(&self) -> ! {
        // Start the first process and get the trap frame.
        let mut tf = TrapFrame::default();
        self.critical(|scheduler| scheduler.switch_to(&mut tf));

        // Setup timer interrupts.
        Controller::new().enable(Interrupt::Timer1);
        IRQ.register(Interrupt::Timer1, Box::new(timer1_handler));
        tick_in(current_time() + TICK);

        // Part of the assignment requirements is to set sp to the address of the "next kernel
        // page" instead of _start before `eret` to have a clean kernel page when there is a fault.
        // It's not clear what exactly is the "next page". In practice, we could have a physical
        // page allocator and allocate one for each process. But this works for now, so I'll leave
        // it.
        unsafe {
            asm!("mov sp, $0
                bl context_restore
                ldr x0, =_start
                mov sp, x0
                mov x0, #0
                eret"
                :: "r"(&tf)
                :: "volatile");
        }

        loop {}
    }

    /// Initializes the scheduler and add userspace processes to the Scheduler
    pub unsafe fn initialize(&self) {
        // Initialize the scheduler.
        *self.0.lock() = Some(Scheduler::new());

        // Add initial userspace processes.
        for _ in 0..3 {
            self.add(Process::load(Path::new("/sleep")).unwrap());
            self.add(Process::load(Path::new("/fib")).unwrap());
        }
    }
}

#[derive(Debug)]
pub struct Scheduler {
    processes: VecDeque<Process>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        Scheduler {
            processes: VecDeque::new(),
            last_id: Some(0),
        }
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
    /// as `Dead` state. Removes the dead process from the queue, drop the
    /// dead process's instance, and returns the dead process's process ID.
    fn kill(&mut self, tf: &mut TrapFrame) -> Option<Id> {
        self.schedule_out(State::Dead, tf);
        match self.processes.pop_back() {
            Some(process) => Some(process.context.tpidr),
            None => None
        }
    }
}

fn timer1_handler(tf: &mut TrapFrame) {
    tick_in(current_time() + TICK);
    crate::SCHEDULER.switch(State::Ready, tf);
}
