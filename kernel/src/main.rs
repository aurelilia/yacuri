#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(yacuri::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use x86_64::VirtAddr;
use yacuri::{
    allocator,
    allocator::{memory, memory::BootInfoFrameAllocator},
    drivers::keyboard,
    graphics::init_graphics,
    hlt_loop, kprintln, println,
    scheduling::{executor::Executor, task::Task},
    vm,
    vm::test_app,
};

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    kprintln!("Hello World! rust says trans rights but with framebuffers now");

    yacuri::init();
    init_graphics(boot_info.framebuffer.as_mut().unwrap());
    init_memory(boot_info);

    test_app();

    #[cfg(test)]
    test_main();

    let mut executor = Executor::new();
    // executor.spawn(Task::new(keyboard::process_keypresses()));
    executor.run();
}

fn init_memory(boot_info: &'static BootInfo) {
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_regions) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
    vm::init_code_heap(&mut mapper, &mut frame_allocator).expect("vm heap initialization failed");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kprintln!("{}", info);
    hlt_loop()
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    yacuri::test_panic_handler(info)
}
