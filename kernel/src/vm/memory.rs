use crate::allocator::prepare_pages;
use alloc::boxed::Box;
use core::{alloc::Layout, ptr::NonNull};
use linked_list_allocator::Heap;
use x86_64::structures::paging::{mapper::MapToError, FrameAllocator, Mapper, Size4KiB};
use yacari::MemoryManager;

pub const CODE_HEAP_START: usize = 0x_6666_6666_0000;
pub const CODE_HEAP_SIZE: usize = 2000 * 1024; // 2MB
pub const PAGE_SIZE: usize = 4096;

struct YacariMemoryManager {
    allocator: linked_list_allocator::Heap,
}

impl YacariMemoryManager {
    /// # Safety
    /// Caller must ensure that the given memory is unused.
    /// Function must be called only once.
    unsafe fn init(heap_start: usize, heap_size: usize) {
        let manager = YacariMemoryManager {
            allocator: Heap::new(heap_start, heap_size),
        };
        yacari::set_manager(Box::new(manager))
    }

    fn layout_from_size(size: usize) -> Layout {
        Layout::from_size_align(size, PAGE_SIZE).unwrap()
    }
}

impl MemoryManager for YacariMemoryManager {
    fn page_size(&self) -> usize {
        PAGE_SIZE
    }

    // TODO: Very safe, much security
    // Pages are allocated RWX by default, so this is 'fine'
    fn set_r(&mut self, _ptr: *mut u8, _size: usize) {}
    fn set_rx(&mut self, _ptr: *mut u8, _size: usize) {}
    fn set_rw(&mut self, _ptr: *mut u8, _size: usize) {}

    fn alloc_page_aligned(&mut self, size: usize) -> *mut u8 {
        self.allocator
            .allocate_first_fit(Self::layout_from_size(size))
            .unwrap()
            .as_ptr()
    }

    fn dealloc(&mut self, ptr: *mut u8, size: usize) {
        unsafe {
            self.allocator
                .deallocate(NonNull::new(ptr).unwrap(), Self::layout_from_size(size))
        }
    }
}

pub fn init_code_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    prepare_pages(mapper, frame_allocator, CODE_HEAP_START, CODE_HEAP_SIZE)?;
    unsafe {
        YacariMemoryManager::init(CODE_HEAP_START, CODE_HEAP_SIZE);
    }
    Ok(())
}
