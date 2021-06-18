#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yacuri::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod vga_buffer;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use yacuri::hlt_loop;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World! rust says trans rights");

    yacuri::init();

    #[cfg(test)]
    test_main();

    hlt_loop()
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
