use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;

use pi::atags::Atags;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        match self.args.len() {
            0 => "",
            _ => self.args[0],
        }
    }
}

const CMD_MAX_CHARS: usize = 512;
const CMD_MAX_ARGS: usize = 64;

fn read_command<'a>(char_buf: &'a mut [u8], cmd_buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
    let mut raw_command = StackVec::new(char_buf);
    let mut num_chars: usize = 0;

    // Keep on accepting characters until we see a newline
    loop {
        let byte = CONSOLE.lock().read_byte();
        match byte {
            // newline
            b'\r' | b'\n' => {
                let cmd = core::str::from_utf8(raw_command.into_slice()).unwrap();
                return Command::parse(cmd, cmd_buf);
            }
            // backspace
            8 | 127 => {
                if num_chars > 0 {
                    kprint!("\u{8} \u{8}");
                    raw_command.pop();
                    num_chars -= 1;
                }
            }
            // visible characters
            32 ..= 126 => {
                if num_chars < CMD_MAX_CHARS {
                    kprint!("{}", byte as char);
                    raw_command.push(byte).unwrap();
                    num_chars += 1;
                }
            }
            // ring the bell on non-visible character
            _ => kprint!("\u{7}"),
        }
    }
}

fn execute_command(cmd: Command) {
    match cmd.path() {
        "echo" => {
            let args = cmd.args.as_slice();
            for i in 1 .. args.len() - 1 {
                kprint!("{} ", args[i]);
            }
            kprintln!("{}", args[args.len() - 1]);
        }
        _ => kprintln!("unknown command: {}", cmd.path()),
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns.
pub fn shell(prefix: &str) -> ! {
    loop {
        let char_buf = &mut [0; CMD_MAX_CHARS];
        let cmd_buf = &mut [""; CMD_MAX_ARGS];

        kprint!("{}", prefix);
        let cmd = read_command(char_buf, cmd_buf);
        kprintln!();

        match cmd {
            Ok(cmd) => execute_command(cmd),
            Err(Error::TooManyArgs) => kprintln!("too many arguments"),
            _ => ()
        }
    }
}
