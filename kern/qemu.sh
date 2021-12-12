#!/bin/sh -x

TOP=$(git rev-parse --show-toplevel)
# $TOP/bin/qemu-system-aarch64 \
$TOP/ext/qemu/build/qemu-system-aarch64 \
    -nographic \
    -M raspi3b \
    -serial null -serial mon:stdio \
    -kernel \
    "$@"
