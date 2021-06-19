use crate::{drivers::vga_buffer, gdt, hlt_loop, print, println};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode, HandlerFunc};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        idt.divide_error.set_handler_fn(generic_fault::<"DIVIDE ERROR">);
        idt.debug.set_handler_fn(generic_fault::<"DEBUG">);
        idt.non_maskable_interrupt.set_handler_fn(generic_fault::<"NON MASKABLE INTERRUPT">);
        idt.overflow.set_handler_fn(generic_fault::<"OVERFLOW">);
        idt.bound_range_exceeded.set_handler_fn(generic_fault::<"BOUND RANGE EXCEEDED">);
        idt.invalid_opcode.set_handler_fn(generic_fault::<"INVALID OPCODE">);
        idt.device_not_available.set_handler_fn(generic_fault::<"DEVICE NOT AVAILABLE">);
        idt.invalid_tss.set_handler_fn(generic_fault_code::<"INVALID TSS">);
        idt.segment_not_present.set_handler_fn(generic_fault_code::<"SEGMENT NOT PRESENT">);
        idt.stack_segment_fault.set_handler_fn(generic_fault_code::<"STACK SEGMENT FAULT">);
        idt.general_protection_fault.set_handler_fn(generic_fault_code::<"GENERAL PROTECTION FAULT">);
        idt.alignment_check.set_handler_fn(generic_fault_code::<"ALIGNMENT CHECK">);
        idt.simd_floating_point.set_handler_fn(generic_fault::<"SIMD FLOATING POINT">);
        idt.virtualization.set_handler_fn(generic_fault::<"VIRTUALIZATION">);
        idt.security_exception.set_handler_fn(generic_fault_code::<"SECURITY EXCEPTION">);

        idt
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        self.as_u8() as usize
    }
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn generic_fault<const NAME: &'static str>(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: {}\n{:#?}", NAME, stack_frame);
}
extern "x86-interrupt" fn generic_fault_code<const NAME: &'static str>(stack_frame: InterruptStackFrame, code: u64) {
    println!("EXCEPTION: {}\n{:#?}\nCODE: {}", NAME, stack_frame, code);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
            Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
        );
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                // Backspace
                DecodedKey::Unicode('\x08') => vga_buffer::clear_last_char(),
                // Tab
                DecodedKey::Unicode('\x09') => print!("    "),

                DecodedKey::Unicode(character) => print!("{}", character),

                DecodedKey::RawKey(KeyCode::ArrowLeft) => vga_buffer::shift_column(-1),
                DecodedKey::RawKey(KeyCode::ArrowRight) => vga_buffer::shift_column(1),
                DecodedKey::RawKey(KeyCode::ArrowUp) => vga_buffer::shift_row(-1),
                DecodedKey::RawKey(KeyCode::ArrowDown) => vga_buffer::shift_row(1),

                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[test_case]
fn test_breakpoint_exception() {
    x86_64::instructions::interrupts::int3();
}
