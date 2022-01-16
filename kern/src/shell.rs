use core::str::FromStr;
use core::time::Duration;

use shim::io::Read;
use shim::path::{Component, PathBuf};

use stack_vec::StackVec;

use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry, File};

use alloc::vec::Vec;

// use kernel_api::syscall::sleep;
use kernel_api::syscall::sleep;
use kernel_api::OsError;

use crate::console::{kprint, kprintln, CONSOLE};
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



struct Shell {
    cwd: PathBuf,
}

impl Shell {

    pub fn new() -> Shell {
        Shell { cwd: PathBuf::from("/") }
    }

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
                        kprint!("\u{8} \u{8}"); raw_command.pop();
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

    // Gets the entries identified by the given path.
    fn get_entry(&self, path: &str) -> PathBuf {
        let mut curr = self.cwd.clone();
        let path = PathBuf::from(path);

        for component in path.components() {
            match component {
                Component::RootDir => curr = PathBuf::from("/"),
                Component::ParentDir => { curr.pop(); },
                Component::Normal(entry) => curr.push(entry),
                _ => (), // Nothing to do for `Prefix` or `CurDir`
            }
        }
        curr
    }

    fn cat(&self, args: &[&str]) {
        if args.len() == 0 {
            kprintln!("expected at least one argument");
        }

        for arg in args {
            match FILESYSTEM.open(self.get_entry(arg)) {
                Ok(entry) => match entry.into_file() {
                    Some(mut file) => {
                        let mut file_contents = Vec::new();
                        for _ in 0..file.size() {
                            file_contents.push(0);
                        }
                        match file.read(file_contents.as_mut_slice()) {
                            Ok(bytes_read) => {
                                if bytes_read < file.size() as usize {
                                    kprintln!("Could only read {} of {} bytes in {}",
                                            bytes_read, file.size(), arg);
                                } else {
                                    match core::str::from_utf8(file_contents.as_slice()) {
                                        Ok(contents) => kprintln!("{}", contents),
                                        Err(_) => kprintln!("{} contains non-UTF8 characters", arg),
                                    }
                                }
                            }
                            Err(_) => kprintln!("Error reading the contents of {}", arg),
                        }
                    },
                    None => kprintln!("{} is a directory", arg),
                }
                Err(_) => kprintln!("Error opening {}", arg),
            }
        }
    }

    fn cd(&mut self, args: &[&str]) {
        if args.len() != 1 {
            kprintln!("cd takes only 1 argument, but received {}", args.len());
            return;
        }

        let arg = args[0];
        if FILESYSTEM.open(self.get_entry(arg)).is_err() {
            kprintln!("Error opening {}", arg);
        } else {
            self.cwd = self.get_entry(arg);
        }
    }

    fn echo(&self, args: &[&str]) {
        for i in 0 .. args.len() - 1 {
            kprint!("{} ", args[i]);
        }
        kprintln!("{}", args[args.len() - 1]);
    }

    fn ls(&self, mut args: &[&str]) {
        let mut display_hidden = false;
        if args.len() > 0 && "-a" == args[0] {
            display_hidden = true;
            args = &args[1..];
        }

        let ls_dir = | path: &PathBuf | {
            match FILESYSTEM.open(path) {
                Ok(entry) => match entry.as_dir() {
                    Some(dir) => {
                        match dir.entries() {
                            Ok(entries) => {
                                for entry in entries {
                                    if display_hidden || !entry.metadata().attributes.hidden() {
                                        kprintln!("{}", entry.name());
                                    }
                                }
                            },
                            Err(_) => kprintln!("Cannot open directory {}", path.to_str().unwrap()),
                        }
                    },
                    None => kprintln!("{}", entry.name()),
                }
                Err(_) => kprintln!("Cannot open directory {}", path.to_str().unwrap()),
            };
        };

        if args.len() == 0 {
            // ls in cwd
            ls_dir(&self.cwd);
        } else {
            // ls each argument
            for arg in args {
                ls_dir(&self.get_entry(arg));
            }
        }
    }

    fn sleep(&self, args: &[&str]) {
        if args.len() < 1 {
            return
        }

        match u64::from_str(args[0]) {
            Ok(millis) => sleep(Duration::from_millis(millis)),
            Err(_) => return,
        }.unwrap();
    }

    fn pwd(&self) {
        kprintln!("{}", self.cwd.to_str().unwrap());
    }

    fn execute_command(&mut self, cmd: Command) -> bool {
        let args = &cmd.args.as_slice()[1..];
        match cmd.path() {
            "cat" => self.cat(args),
            "cd" => self.cd(args),
            "echo" => self.echo(args),
            "exit" => return false,
            "ls" => self.ls(args),
            "sleep" => self.sleep(args),
            "pwd" => self.pwd(),
            _ => kprintln!("unknown command: {}", cmd.path()),
        }
        true
    }

    pub fn shell(&mut self, prefix: &str) {
        loop {
            let char_buf = &mut [0; CMD_MAX_CHARS];
            let cmd_buf = &mut [""; CMD_MAX_ARGS];

            kprint!("{}", prefix);
            let cmd = Shell::read_command(char_buf, cmd_buf);
            kprintln!();

            match cmd {
                Ok(cmd) => {
                    if !self.execute_command(cmd) {
                        return
                    }
                }
                Err(Error::TooManyArgs) => kprintln!("too many arguments"),
                _ => ()
            }
        }
    }
}

pub fn shell(prefix: &str) {
    let mut shell = Shell::new();
    shell.shell(prefix)
}
