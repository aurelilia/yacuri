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

```bash 
# Execute in QEMU
cd kernel; cargo run

# Run tests
cd kernel; cargo test; cd ../lang; cargo test
```