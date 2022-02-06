#!/bin/bash

set -e

TOP=$(git rev-parse --show-toplevel)
BIN=$TOP/bin
EXT=$TOP/ext
# Could probably just check out head if this does not work.
# version of QEMU than the course, and the ID got updated.
git apply $BIN/qemu.patch
VER=a3607def89f9cd68c1b994e1030527df33aa91d0

cd $EXT

if [[ ! -e qemu-system-aarch64 ]]; then
    git clone https://github.com/qemu/qemu

    cd qemu
    git checkout $VER -b cs3210
    git submodule init
    git submodule update

    mkdir -p build
    cd build
    ../configure --disable-capstone --target-list=aarch64-softmmu --disable-werror
    make -j$($BIN/ncpus)
fi
