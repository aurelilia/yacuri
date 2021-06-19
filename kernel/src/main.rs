#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yacuri::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use yacuri::{hlt_loop, print, println, allocator, memory};
use yacuri::drivers::disk::ata_pio::AtaBus;
use fatfs::Read;
use alloc::boxed::Box;
use x86_64::VirtAddr;
use yacuri::memory::BootInfoFrameAllocator;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World! rust says trans rights");

    yacuri::init();
    init_memory(boot_info);

    let mut blockdev = AtaBus::new(0x1F0, 0x3F6);
    let mut buf = Box::new([1; 512]);
    blockdev.read(&mut *buf);
    println!("{:?}", buf[0]);

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
