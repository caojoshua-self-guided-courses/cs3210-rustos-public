# SD Card Issues

The course links the kernel against `kern/.cargo/.libsd.a`, which provides `sd_init()`, and `sd_readsector()`. We define a `wait_micros()` in rust, which the procedures in `libsd.a` call to wait for SD card responses. The `libsd.a` source is not provided, and I could not find it anywhere online. It is difficult to understand the disassembled code in the library.

When I tried running `sd_init()` I was consistently getting SD card operation timeouts. Looking at the [CS140E gitter](https://gitter.im/cs140e-rust/Lobby?source=orgpage), many people multiplied the wait time by various factors ie. 20, 100, 1000. I tried many different wait times, but I kept on getting timeouts or other errors.

I tried running the kernel from two other people who had completed the course and published it on GitHub. Running their kernels presented the same SD card timeout, which meant that my code was probably fine and the problem was in my hardware.

I initially was using a Samsung 32 GB SD card. I saw someone in the gitter switched from a 32 to 16 GB card to get their kernel working. I ordered a new Sandisk 16 GB card, tested it out, and was running into the same exact issues.

My next thought was that maybe `libsd.a` is not implemented well. I spent 30 minutes trying to write my own EMMC/SD driver from scratch, but did not make any progress. Judging from the source code of existing EMMC driver implementations, I knew it would not be very simple. Instead, I pulled the SD source code from [raspi3-tutorial](https://github.com/bztsrc/raspi3-tutorial/tree/master/0B_readsector) in [this commit](https://github.com/caojoshua-self-guided-courses/cs3210-rustos-public/commit/bba9adde248febe03ee8d7ca177ba02b2a1ff306). I got it linked and working in the kernel in [this commit](https://github.com/caojoshua-self-guided-courses/cs3210-rustos-public/commit/bba9adde248febe03ee8d7ca177ba02b2a1ff306). To get it working, I had to modify the external SD code to call the rust `wait_micros`, which needed to multiply the wait time by a constant.

Unfortunately, `sd_init()` was still timing out inconsistently. It seemed that whenever I had not run the kernel for a while, `sd_init()` will timeout, but after a few tries, it would work fine consistently. My guess is that the Pi needed to 'warm up' for some reason. I tried working like this for a while, but quickly found it was not a pleasant experience.

Finally, I decided to try running the kernel in QEMU. The course never stated when would be a good point to move over from bare metal to QEMU, and the default scripts were not working, so I was too lazy to get QEMU set up initially. After spending 15 minutes getting QEMU setup in [this commit](https://github.com/caojoshua-self-guided-courses/cs3210-rustos-public/commit/93a364887fcb6b2572677cff7a9d43c9d5312411), I had the kernel working flawlessly, without the need for increasing the wait time. I'm still using the external SD code and removed the course provided `libsd.a`, just because its nice to have the source code, and I'm too lazy to change whats already working.

So just like that, a weekend of work, a week of waiting, and another 1-2 days of debugging this issue, was solved by 15 minutes of work. Lesson learned: always stick to software when possible and avoid working with hardware.
