/// Zero global allocator.
pub struct NoAllocator;

unsafe impl core::alloc::GlobalAlloc for NoAllocator {
    #[inline(always)]
    unsafe fn alloc(&self, _: core::alloc::Layout) -> *mut u8 {
        panic!("** NO ALLOCATOR **");
    }

    #[inline(always)]
    unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {
        // I deny all allocations, so I don't need to free.
    }
}
