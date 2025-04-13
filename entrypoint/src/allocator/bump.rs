pub struct BumpAllocator {
    start: usize,
    len: usize,
}

impl BumpAllocator {
    #[inline(always)]
    pub const fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }
}

unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
    /// Allocates memory as a bump allocator.
    #[inline(always)]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let pos_ptr = self.start as *mut usize;

        let mut pos = *pos_ptr;
        if pos == 0 {
            // First time, set starting position.
            pos = self.start + self.len;
        }
        pos = pos.saturating_sub(layout.size());
        pos &= !(layout.align().wrapping_sub(1));
        if pos < self.start + core::mem::size_of::<*mut u8>() {
            return core::ptr::null_mut();
        }
        *pos_ptr = pos;
        pos as *mut u8
    }

    #[inline(always)]
    unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {
        // I'm a bump allocator, I don't free.
    }
}
