#!/bin/sh -x

TOP=$(git rev-parse --show-toplevel)
# $TOP/bin/qemu-system-aarch64 \
sudo $TOP/ext/qemu/build/qemu-system-aarch64 \
    -nographic \
    -M raspi3b,usb=on \
    -serial null -serial mon:stdio \
    -kernel \
    "$@"

# sudo /home/josh/src/qemu/build/aarch64-softmmu/qemu-system-aarch64 \
    # -netdev user,id=mynet0,hostfwd=tcp::8080-:80 -device e1000,netdev=mynet0 \
    # -netdev user,id=net0,hostfwd=tcp::8080-:80 -device usb-net,netdev=net0
    # -netdev bridge,br=foobar,id=net1 -device e1000,netdev=net1,id=nic1 \
