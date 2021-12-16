# GATECH CS3210 Operating Systems
Fork of [Georgia Tech CS3210 Operating Systems](https://github.com/sslab-gatech/cs3210-rustos-public). I am following the 2020 Spring course [[website](https://tc.gts3.org/cs3210/2020/spring/index.html). I was initially following [Stanford CS140E](https://cs140e.sergio.bz/), but found that this course is an extension of the former with more content and a newer rust toolchain. I have also used the [CS140E gitter](https://gitter.im/cs140e-rust/Lobby?source=orgpage) as a resource.

Motivations for taking this course:
* learn Rust, and this seemed like a decent place to start
* learn more about OS. I took [Stanford CS140 Operating
  Systems](https://github.com/caojoshua-self-guided-courses/StanfordCS140) last year. It was a really good learning experience implementing OS concepts ie. threading, userspace, virtual memory, filesystems, but it did not cover direct hardware interactions as much. This class includes driver implementations and interfacing with special registers, which would be a first for me. As a bonus, this course also includes a Networking stack implementation. It seems that CS3210 can pack more in because its a Semester class, versus Stanford CS140 being a quarter class.

I provide instructions on how to build/run the kernel below, although much of it is repeated from the course. Disclaimer: I had a lot of issues with the build/run scripts in this course. I modified them such that they work for me, but they may break on other machines. I am working on this course from both Fedora and Arch Linux machines, and have not tested on anything else.

## Hardware
My hardware is the same as those listed in the Assignment 1. Specifics:
* Raspberry Pi 3 Model B
* 16 GB Sandisk SD card and 32 GB Samsung SD Card
* random CP2102 I got online
* breadboard, resistors, LEDs, jumper cables, etc. I got from a friend.

This setup all worked fine until the end of Assignment 3 where I faced SD card issues (see [sdcard-issues.md](https://github.com/caojoshua-self-guided-courses/cs3210-rustos-public/docs/sdcard-issues.md))

### Formatting the SD card
To get the name of your device:
```
lsblk
```

Then format the device (this will wipe contents):
```
sudo parted /dev/sdb --script -- mklabel msdos
```

Then build the Fat32 partition:
```
sudo mkfs.vfat -F32 /dev/sdb1 -s 1 -S 512
```

`-s 1` will set `sectors per cluster = 1`. For some reason when I ran this the first time, it was set to 64, which was causing memory issues.

## Building
First run:
```
bin/setup.sh
```

To build the bootloader:
```
cd boot
make
```
which builds the artifacts in `boot/build`

And to build the kernel:
```
cd kern
make
```
which builds the artifacts in `kern/build`

I had a lot of issues building dependencies due to a lack of `Cargo.lock` files in `lib/` (the instructors intentionally left it out for somereason). It works now, but as older cargo packages get yanked, they may break again in the future. If this is the case, you would need to mess around with the dependency versions. I have not tried this, but it could also work to use the VM image provided by the course.

## Running on bare metal

The kernel can either be installed directly onto the Pi, or sent to the Pi and loaded by the bootloader. Due to issues with the SD card, it is recommended to skip this section and run the kernel on QEMU.

### Running the kernel directly from the pi
Mount your SD card. Then run:
```
sudo bin/install-kernel.py kern/build/kernel.elf <SD card partition name ie. /dev/sdb1>
```

Plug in the CP2102 into a computer's USB port. Then run:
```
sudo screen /dev/ttyUSB0 115200  # ttyUSB0 might be something else on different machines
```

This will open up a terminal where you can see the outputs of the terminal.

### Running the kernel through the bootloader
Mount your SD card. Then run:
```
sudo bin/install-kernel.py boot/build/kernel.elf <SD card partition name ie. /dev/sdb1>
```

Plug in the CP2102 into a computer's USB port. Then run:
```
cd kern
make transmit
```

This will send the kernel to the Pi over the Xmodem protocol, and open up a screen session which displays the outputs of the kernel.

## Running in QEMU
This is the best approach to run the kernel, as it does not face harware issues.

First build QEMU:
```
bin/build-qemu.sh
```
I modified the script to check out a commit that works for me. It could be better to just build HEAD, or use a release version instead.

To run the kernel:
```
cd kern
make qemu
```

This will execute QEMU with the kernel loaded in the current terminal.
