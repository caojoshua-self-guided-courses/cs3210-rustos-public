use core::time::Duration;
use core::panic::PanicInfo;
use crate::console::kprintln;
use pi::timer::spin_sleep;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    for _ in 0 .. 5 {
        kprintln!("{}", _info);
        spin_sleep(Duration::from_secs(1));
    }

    loop {}
}
