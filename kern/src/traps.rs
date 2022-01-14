mod frame;
mod syndrome;
mod syscall;

pub mod irq;
pub use self::frame::TrapFrame;

use pi::interrupt::{Controller, Interrupt};

use crate::console::kprintln;
use crate::IRQ;

use self::syndrome::Syndrome;
use self::syscall::handle_syscall;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
/// Adding far register, which for address related faults, holds the address of the fault.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, far: u64, tf: &mut TrapFrame) {
    if info.kind == Kind::Synchronous {
        let syndrome = Syndrome::from(esr);
        match syndrome {
            Syndrome::Svc(num) => handle_syscall(num, tf),
            _ => {
                // Print out info for non syscall synchronous exceptions.
                kprintln!("handle_exception: {:#?}", info);
                kprintln!("syndrome: {:#?}", syndrome);
                kprintln!("fault addr: {:x}", far);
                crate::shell::shell("exception > ");

                // We increment the PC only for non-syscall synchronous calls because we want to
                // jump to the instruction after the one that generated the exception.
                tf.increment_link_addr(4);
            },
        }

    } else if info.kind == Kind::Irq {
        let controller = Controller::new();
        for interrupt in Interrupt::iter() {
            if controller.is_pending(*interrupt) {
                IRQ.invoke(*interrupt, tf);
            }
        }
    } else {
        kprintln!("handle_exception: {:#?}", info);
        loop {
            aarch64::nop();
        }
    }
}
