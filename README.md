# yacuri

A WIP kernel and programming language with a novel approach to security.

Currently pretty early, most of the kernel fundamentals are based on [this great blog series](https://os.phil-opp.com/).

# Features

- Support for FAT filesystems attached via ATA PIO
- Custom allocator
- VGA text mode shell with a few commands (ls, cat, mkdir)
- Basic async executor/runtime

# Setup & Run

Due to suboptimal cargo support for custom targets, trying to run anything
in the parent directory will fail.

Before being able to use the kernel, you need to prepare the needed disks. `kernel/init.sh`
can do this for you.

Additionally, `cargo krun` currently requires OVMF UEFI firmware. If the path
of yours differs, modify `RUN_ARGS`  and `TEST_ARGS` in `kernel/bootimage/src/main.rs`.
(TODO: Maybe package the firmware? this is not a solution.)

Finally, make sure you clone git submodules with `git submodule init` followed by `git submodule update`.

```bash 
# Execute in QEMU
cd kernel; cargo krun

# Run tests
cd kernel; cargo ltest; cd ../lang; cargo test
```