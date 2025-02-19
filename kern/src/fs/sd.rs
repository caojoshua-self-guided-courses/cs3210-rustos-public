use shim::io;

use fat32::traits::BlockDevice;

extern "C" {
    /// A global representing the last SD controller error that occured.
    pub static sd_err: i64;

    /// Initializes the SD card controller.
    ///
    /// Returns 0 if initialization is successful. If initialization fails,
    /// returns -1 if a timeout occured, or -2 if an error sending commands to
    /// the SD controller occured.
    fn sd_init() -> i32;

    /// Reads sector `n` (512 bytes) from the SD card and writes it to `buffer`.
    /// It is undefined behavior if `buffer` does not point to at least 512
    /// bytes of memory. Also, the caller of this function should make sure that
    /// `buffer` is at least 4-byte aligned.
    ///
    /// On success, returns the number of bytes read: a positive number.
    ///
    /// On error, returns 0. The true error code is stored in the `sd_err`
    /// global. `sd_err` will be set to -1 if a timeout occured or -2 if an
    /// error sending commands to the SD controller occured. Other error codes
    /// are also possible but defined only as being less than zero.

    // We are using an external libsd.a, so we cannot use the following.
    // fn sd_readsector(n: i32, buffer: *mut u8) -> i32;

    // External libsd.a equivalent to sd_readsector. This API also allows
    // reading multiple sectors.
    fn sd_readblock(sector_num: u32, buffer: *mut u8, num_sector: u32) -> i32;
}

// FIXME: Define a `#[no_mangle]` `wait_micros` function for use by `libsd`.
// The `wait_micros` C signature is: `void wait_micros(unsigned int);`
#[no_mangle]
fn wait_micros(us: u32) {
    // Bare metal requires a wait time.
    // let us = us * 2000;
    pi::timer::spin_sleep(core::time::Duration::from_micros(us.into()));
}

// External libsd functions
#[no_mangle]
fn uart_puts(_bytes: *const [u8]) {
}
#[no_mangle]
fn uart_hex(_hex: u32) {
}

/// A handle to an SD card controller.
#[derive(Debug)]
pub struct Sd;

impl Sd {
    /// Initializes the SD card controller and returns a handle to it.
    /// The caller should assure that the method is invoked only once during the
    /// kernel initialization. We can enforce the requirement in safe Rust code
    /// with atomic memory access, but we can't use it yet since we haven't
    /// written the memory management unit (MMU).
    pub unsafe fn new() -> Result<Sd, io::Error> {
        match sd_init() {
            0 => Ok(Sd),
            error_code => Err(Sd::err(error_code.into())),
        }
    }

    fn err(error_code: i64) -> io::Error {
        crate::console::kprintln!("error code {}", error_code);
        match error_code {
            0 => io::Error::new(io::ErrorKind::Other,
                "not an error"),
            -1 => io::Error::new(io::ErrorKind::TimedOut,
                "timed out on SD card operation"),
            -2 => io::Error::new(io::ErrorKind::Other,
                "error sending commands to the SD card"),
            _ => io::Error::new(io::ErrorKind::Other,
                "unknown error"),
        }
    }
}

impl BlockDevice for Sd {
    /// Reads sector `n` from the SD card into `buf`. On success, the number of
    /// bytes read is returned.
    ///
    /// # Errors
    ///
    /// An I/O error of kind `InvalidInput` is returned if `buf.len() < 512` or
    /// `n > 2^31 - 1` (the maximum value for an `i32`).
    ///
    /// An error of kind `TimedOut` is returned if a timeout occurs while
    /// reading from the SD card.
    ///
    /// An error of kind `Other` is returned for all other errors.
    fn read_sector(&mut self, n: u64, buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() < 512 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput,
                "buffer size is less than 512"))
        } else if n > 0xFFFFFFFF {
            return Err(io::Error::new(io::ErrorKind::InvalidInput,
                "reading from sector number > 0xFFFFFFFF"))
        }

        unsafe {
            match sd_readblock(n as u32, buf.as_mut_ptr(), 1) {
                0 => Err(Sd::err(sd_err)),
                _ => Ok(512)
            }
        }
    }

    fn write_sector(&mut self, _n: u64, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("SD card and file system are read only")
    }
}
