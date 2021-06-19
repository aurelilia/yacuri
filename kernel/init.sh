#!/bin/sh
dd if=/dev/urandom of=src/drivers/disk/test_drive.bin bs=1024 count=64
dd if=/dev/zero of=fs.bin bs=1024 count=1024
mkfs.fat fs.bin
