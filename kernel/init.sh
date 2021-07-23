#!/bin/sh
dd if=/dev/urandom of=src/drivers/disk/test_drive.bin bs=1024 count=64
dd if=/dev/zero of=fs.bin bs=1024 count=1024
mkfs.fat fs.bin

mkdir -p /tmp/fatfs
sudo mount fs.bin /tmp/fatfs -o loop,uid=$(id -u)
cp -r install_fs/* /tmp/fatfs/
sudo umount /tmp/fatfs