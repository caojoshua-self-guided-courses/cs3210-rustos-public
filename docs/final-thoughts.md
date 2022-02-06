# Final Thoughts

This course serves as a decent learning experience to Rust and Operating Systems. I got a taste of how difficult Rust is, and got to work with lower level OS concepts I had not before. The most interesting parts were lab 4 and the multicore part of lab5, since it involves working with core kernel CPU/memory concepts such as threads, processes, multicore, and virtual memory. The other labs were mostly drivers and userspace tooling, which is also interesting in its own way since I had not done similar work before.

My opinion on the limitations are:
* Rust is hard, and working on Rust in a OS is harder. We have to make extensive use of advanced features like macros, nostd, and unsafe Rust. Learning Rust in a userspace may be better for learning the language.
* The project is not well maintained. A lot of the scripts, make targets, and pre-provided binaries don't work. Cargo.lock is not included for a lot of the libraries, so I needed to fix a lot of builds. There are also hardware/QEMU issues such as USB and SD card issues which I faced.
* The a million merge conflicts when merging in lab 5 were unnecessary. This took a lot of time, and most it should have been included in lab 4.
* Lab 4 and especially Lab 5 assignment pages don't have great instructions and a lot of explantaions feel short and missing details. It could be that these labs were rushed, since labs 0-3 were taken from [Stanford CS140e](https://cs140e.sergio.bz/) and labs 4-5 needed to be created from scratch.

I personally would not recommend this course. If I had known ahead of time, I would have gone with one of these better maintained Rust OS options:
* [rust-raspberrypi-OS-tutolrials](https://github.com/rust-embedded/rust-raspberrypi-OS-tutorials)
* [rpi4-osdev](https://github.com/isometimes/rpi4-osdev)

The final diff from main branch(run with `git diff --stat origin/lab5 -- ':(exclude)*.c' ':(exclude)*.h'` from repo root)

```
 .gitignore                                         |   2 +
 README.md                                          | 127 +++++++--
 bin/build-qemu.sh                                  |   7 +-
 bin/install-kernel.py                              |   4 +-
 bin/setup.sh                                       |   2 +-
 boot/Cargo.lock                                    |  74 ++++++
 boot/src/main.rs                                   |  22 +-
 docs/sdcard-issues.md                              |  17 ++
 kern/.cargo/libsd.a                                | Bin 4780 -> 0 bytes
 kern/Cargo.lock                                    |  28 +-
 kern/Makefile                                      |  21 +-
 kern/qemu.sh                                       |   7 +-
 kern/sd/.gitignore                                 |   2 +
 kern/sd/Makefile                                   |   8 +
 kern/src/allocator.rs                              |  16 +-
 kern/src/allocator/bin.rs                          | 155 ++++++++++-
 kern/src/allocator/bump.rs                         |  16 +-
 kern/src/allocator/util.rs                         |  31 ++-
 kern/src/console.rs                                |  18 +-
 kern/src/fs.rs                                     |  29 ++-
 kern/src/fs/sd.rs                                  |  60 ++++-
 kern/src/init.rs                                   |  31 ++-
 kern/src/init/panic.rs                             |   3 +
 kern/src/init/vectors.s                            | 142 ++++++++++-
 kern/src/main.rs                                   |  11 +-
 kern/src/mutex.rs                                  |  29 ++-
 kern/src/param.rs                                  |   4 +-
 kern/src/process/process.rs                        |  89 ++++++-
 kern/src/process/scheduler.rs                      | 144 +++++++++--
 kern/src/shell.rs                                  | 242 +++++++++++++++++-
 kern/src/traps.rs                                  |  47 +++-
 kern/src/traps/frame.rs                            |  18 +-
 kern/src/traps/irq.rs                              |   8 +-
 kern/src/traps/syndrome.rs                         |  48 +++-
 kern/src/traps/syscall.rs                          |  64 ++++-
 kern/src/vm.rs                                     |  21 +-
 kern/src/vm/pagetable.rs                           | 157 ++++++++++--
 lib/fat32/Cargo.toml                               |   8 +-
 lib/fat32/src/mbr.rs                               |  77 +++++-
 lib/fat32/src/tests.rs                             |  15 ++
 lib/fat32/src/traits/fs.rs                         |   2 +
 lib/fat32/src/vfat/cache.rs                        |  43 +++-
 lib/fat32/src/vfat/cluster.rs                      |   2 +-
 lib/fat32/src/vfat/dir.rs                          | 215 +++++++++++++++-
 lib/fat32/src/vfat/ebpb.rs                         |  92 ++++++-
 lib/fat32/src/vfat/entry.rs                        |  49 +++-
 lib/fat32/src/vfat/fat.rs                          |  19 +-
 lib/fat32/src/vfat/file.rs                         |  70 ++++-
 lib/fat32/src/vfat/metadata.rs                     | 132 +++++++++-
 lib/fat32/src/vfat/vfat.rs                         | 283 ++++++++++++++++++---
 lib/kernel_api/src/lib.rs                          |   4 +-
 lib/kernel_api/src/syscall.rs                      |  47 +++-
 lib/pi/src/atags/atag.rs                           |  40 ++-
 lib/pi/src/atags/mod.rs                            |  13 +-
 lib/pi/src/atags/raw.rs                            |   8 +-
 lib/pi/src/gpio.rs                                 |  22 +-
 lib/pi/src/interrupt.rs                            |  34 ++-
 lib/pi/src/local_interrupt.rs                      |  79 +++++-
 lib/pi/src/timer.rs                                |  16 +-
 lib/pi/src/uart.rs                                 | 110 +++++++-
 lib/stack-vec/src/lib.rs                           |  69 ++++-
 lib/ttywrite/Cargo.toml                            |   2 +
 lib/ttywrite/src/main.rs                           |  25 +-
 lib/xmodem/src/lib.rs                              |  94 ++++++-
 .../exercises/conversions/as_ref_mut.rs            |   8 +-
 tut/0-rustlings/exercises/conversions/from_into.rs |  14 +-
 tut/0-rustlings/exercises/conversions/from_str.rs  |  13 +-
 .../exercises/conversions/try_from_into.rs         |  13 +-
 tut/0-rustlings/exercises/conversions/using_as.rs  |   5 +-
 tut/0-rustlings/exercises/cs140e/borrow-1.rs       |   3 +-
 tut/0-rustlings/exercises/cs140e/borrow-2.rs       |   8 +-
 tut/0-rustlings/exercises/cs140e/builder.rs        |  27 +-
 tut/0-rustlings/exercises/cs140e/const.rs          |   4 +-
 tut/0-rustlings/exercises/cs140e/derive.rs         |   3 +-
 tut/0-rustlings/exercises/cs140e/expressions.rs    |  10 +-
 tut/0-rustlings/exercises/cs140e/feature-1.rs      |   2 +-
 tut/0-rustlings/exercises/cs140e/io-read-write.rs  |   6 +-
 tut/0-rustlings/exercises/cs140e/lifetimes-1.rs    |   4 +-
 tut/0-rustlings/exercises/cs140e/lifetimes-2.rs    |   6 +-
 tut/0-rustlings/exercises/cs140e/lifetimes-3.rs    |   6 +-
 tut/0-rustlings/exercises/cs140e/lifetimes-4.rs    |   8 +-
 tut/0-rustlings/exercises/cs140e/mutability-1.rs   |   6 +-
 tut/0-rustlings/exercises/cs140e/mutability-2.rs   |   4 +-
 tut/0-rustlings/exercises/cs140e/mutability-3.rs   |   4 +-
 tut/0-rustlings/exercises/cs140e/mutability-4.rs   |   6 +-
 .../exercises/cs140e/pattern-match-1.rs            |   6 +-
 tut/0-rustlings/exercises/cs140e/privacy.rs        |   4 +-
 tut/0-rustlings/exercises/cs140e/semi.rs           |   4 +-
 tut/0-rustlings/exercises/cs140e/trait-impl.rs     |  21 +-
 .../exercises/cs140e/trait-namespace.rs            |   4 +-
 tut/0-rustlings/exercises/cs140e/try.rs            |   8 +-
 tut/0-rustlings/exercises/cs140e/ufcs.rs           |   6 +-
 tut/0-rustlings/exercises/enums/enums1.rs          |  15 +-
 tut/0-rustlings/exercises/enums/enums2.rs          |   7 +-
 tut/0-rustlings/exercises/enums/enums3.rs          |  12 +-
 .../exercises/error_handling/errors1.rs            |  13 +-
 .../exercises/error_handling/errors2.rs            |   4 +-
 .../exercises/error_handling/errors3.rs            |   6 +-
 .../exercises/error_handling/errorsn.rs            |  12 +-
 .../exercises/error_handling/option1.rs            |   4 +-
 .../exercises/error_handling/result1.rs            |  10 +-
 tut/0-rustlings/exercises/functions/functions1.rs  |   2 +-
 tut/0-rustlings/exercises/functions/functions2.rs  |   4 +-
 tut/0-rustlings/exercises/functions/functions3.rs  |   4 +-
 tut/0-rustlings/exercises/functions/functions4.rs  |   4 +-
 tut/0-rustlings/exercises/functions/functions5.rs  |   4 +-
 tut/0-rustlings/exercises/if/if1.rs                |   7 +-
 tut/0-rustlings/exercises/iterators/arc1.rs        |  45 ++++
 tut/0-rustlings/exercises/iterators/iterators2.rs  |   8 +-
 tut/0-rustlings/exercises/iterators/iterators3.rs  |  18 +-
 tut/0-rustlings/exercises/iterators/iterators4.rs  |   7 +-
 tut/0-rustlings/exercises/macros/macros1.rs        |   4 +-
 tut/0-rustlings/exercises/macros/macros2.rs        |  10 +-
 tut/0-rustlings/exercises/macros/macros3.rs        |   3 +-
 tut/0-rustlings/exercises/macros/macros4.rs        |   4 +-
 tut/0-rustlings/exercises/modules/modules1.rs      |   4 +-
 tut/0-rustlings/exercises/modules/modules2.rs      |   6 +-
 .../exercises/move_semantics/move_semantics1.rs    |   4 +-
 .../exercises/move_semantics/move_semantics2.rs    |   4 +-
 .../exercises/move_semantics/move_semantics3.rs    |   4 +-
 .../exercises/move_semantics/move_semantics4.rs    |   8 +-
 .../exercises/primitive_types/primitive_types1.rs  |   4 +-
 .../exercises/primitive_types/primitive_types2.rs  |   4 +-
 .../exercises/primitive_types/primitive_types3.rs  |   4 +-
 .../exercises/primitive_types/primitive_types4.rs  |   4 +-
 .../exercises/primitive_types/primitive_types5.rs  |   4 +-
 .../exercises/primitive_types/primitive_types6.rs  |   4 +-
 tut/0-rustlings/exercises/strings/strings1.rs      |   4 +-
 tut/0-rustlings/exercises/strings/strings2.rs      |   4 +-
 tut/0-rustlings/exercises/structs/structs1.rs      |  13 +-
 tut/0-rustlings/exercises/structs/structs2.rs      |   8 +-
 tut/0-rustlings/exercises/test1.rs                 |  10 +-
 tut/0-rustlings/exercises/test2.rs                 |  22 +-
 tut/0-rustlings/exercises/test3.rs                 |   5 +-
 tut/0-rustlings/exercises/test4.rs                 |   6 +-
 tut/0-rustlings/exercises/tests/tests1.rs          |   4 +-
 tut/0-rustlings/exercises/tests/tests2.rs          |   4 +-
 tut/0-rustlings/exercises/tests/tests3.rs          |   9 +-
 tut/0-rustlings/exercises/variables/variables1.rs  |   4 +-
 tut/0-rustlings/exercises/variables/variables2.rs  |   4 +-
 tut/0-rustlings/exercises/variables/variables3.rs  |   4 +-
 tut/0-rustlings/exercises/variables/variables4.rs  |   4 +-
 tut/1-blinky/phase4/src/main.rs                    |  10 +-
 143 files changed, 3269 insertions(+), 594 deletions(-)
```
