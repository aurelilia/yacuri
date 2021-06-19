#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yacuri::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use yacuri::{hlt_loop, print, println, allocator, memory};
use yacuri::drivers::disk::ata_pio::AtaDrive;
use fatfs::{Read, Write};
use alloc::boxed::Box;
use x86_64::VirtAddr;
use yacuri::memory::BootInfoFrameAllocator;
use yacuri::drivers::disk::fat::{fat_from_ata, fat_from_secondary};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World! rust says trans rights");

    yacuri::init();
    init_memory(boot_info);

    let fs = fat_from_secondary();
    {
        let root_dir = fs.root_dir();
        root_dir.create_dir("helloWorld").unwrap();
        let mut file = root_dir.create_file("helloWorld/yay").unwrap();
        file.truncate().unwrap();
        file.write_all(b"you did it!");

        let dir = root_dir.open_dir("helloWorld").unwrap();
        for r in dir.iter() {
            let entry = r.unwrap();
            println!("{}", entry.file_name());
        }
    }
    fs.unmount().unwrap();

    #[cfg(test)]
    test_main();

    hlt_loop()
}

fn init_memory(boot_info: &'static BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    yacuri::test_panic_handler(info)
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion... ");
    assert_eq!(1, 1);
    println!("[ok]");
}
