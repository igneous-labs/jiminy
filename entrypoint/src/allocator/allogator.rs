//! A bump allocator that supports compile-time allocations
//! by simply starting the heap at runtime at some offset from default starting position
//! while using the offset bytes as memory accessible at compile time.
//!
//! Code adapted from cavemanloverboy, licensed under Apache-2.0:
//! https://github.com/cavemanloverboy/allogator

use core::{
    alloc::{GlobalAlloc, Layout},
    mem::{align_of, size_of},
    num::NonZeroUsize,
    ptr::null_mut,
};

use super::HEAP_START_ADDRESS;

const DEFAULT_HEAP_LENGTH: usize = super::HEAP_LENGTH;

/// Users can request for more heap size in multiples of 1024
/// using `ComputeBudgetInstruction::request_heap_frame`.
/// Increase the `HEAP_LENGTH` const generic if so.
///
/// # Implementation
///
/// The heap grows downwards from `HEAP_END = HEAP_START_ADDRESS + HEAP_LENGTH` to
/// `HEAP_START_ADDRESS`. Growing downwards instead of upwards
/// is for easy arithmetic to round to alignment
/// of allocations by just zeroing out the approriate lower bits.
/// The cursor address therefore decreases with each new allocation.
///
/// The struct contains a `const_heap_end: usize` that is the pointer to the lowest address
/// of the compile-time allocated blocks of memory. Runtime allocations can occur
/// downwards starting below this address.
///
/// A `cursor: usize` is stored at `*(HEAP_END - 8)`
/// that stores the pointer to the current end of the heap.
/// This is 0 at entrypoint time, and is initialized to
/// `self.const_heap_end` before the first allocation
///
/// Using usize instead of `*mut u8` because pointers cannot be cast
/// to integers during const-eval. We dont have strict provenance of pointers
/// but `usize as *mut u8` should be equivalent to `with_provenance_mut`
/// https://doc.rust-lang.org/std/ptr/fn.with_exposed_provenance_mut.html
/// Tho still not sure if this avoid UB completely.
///
/// We also cannot store any mutable state in this struct, even via interior mutability types,
/// because doing so causes this to be put into static writable memory, which is not
/// allowed by the runtime. Not sure if this is the right cause but compiled binaries
/// using this approach have proven to be not executable.
#[derive(Debug)] // do not derive Copy and Clone to avoid possibly having 2 allocators
#[repr(transparent)]
pub struct Allogator<const HEAP_LENGTH: usize = DEFAULT_HEAP_LENGTH> {
    const_heap_end: usize,
}

/// Returns `pointer_to_new_alloc`.
///
/// Any cursors keeping track of start of new memory should be updated to this value as well.
///
/// Returns `0` (null pointer) if OOM.
///
/// Mutation of self needs to be split out because dereferencing *mut is not yet
/// stable on cargo-build-sbf's rustc 1.79.
///
/// Need to use usize instead of pointers so that its usable in const-contexts
#[inline(always)]
const fn alloc_result(cursor: usize, layout: Layout) -> Option<NonZeroUsize> {
    let res = cursor.saturating_sub(layout.size())
    // & !(align - 1) zeros out low bits so cursor is aligned to layout
    // which is a power of 2
    & !(layout.align().wrapping_sub(1));

    // out of heap memory
    if res < HEAP_START_ADDRESS {
        // null pointer
        None
    } else {
        // safety: res >= HEAP_START_ADDRESS
        Some(unsafe { NonZeroUsize::new_unchecked(res) })
    }
}

impl<const HEAP_LENGTH: usize> Allogator<HEAP_LENGTH> {
    const HEAP_END: usize = HEAP_START_ADDRESS + HEAP_LENGTH;
    const CURSOR_ADDR: usize = Self::HEAP_END - size_of::<usize>();

    #[inline]
    pub const fn new() -> Self {
        const {
            assert!(HEAP_LENGTH % 1024 == 0);
            assert!(HEAP_LENGTH > 0);
            assert!(Self::CURSOR_ADDR % align_of::<usize>() == 0);
        }

        Self {
            const_heap_end: Self::CURSOR_ADDR,
        }
    }

    /// This method is stricly meant to be used at compile-time only.
    ///
    /// Returns (updated Allogator, compile-time allocation).
    ///
    /// To avoid UB, the returned compile-time allocation should
    /// always be `MaybeUninit` or a type that can be zero-initialized.
    ///
    /// No mutations allowed in const in rustc 1.79 yet
    /// so just do self -> Self.
    #[inline]
    pub const fn const_alloc(self, layout: Layout) -> (Self, *mut u8) {
        let Self { const_heap_end } = self;
        let const_heap_end = match alloc_result(const_heap_end, layout) {
            None => panic!("Heap OOM"),
            Some(c) => c.get(),
        };
        (Self { const_heap_end }, const_heap_end as *mut u8)
    }
}

unsafe impl<const HEAP_LENGTH: usize> GlobalAlloc for Allogator<HEAP_LENGTH> {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // deref of this ptr is most likely UB
        let cursor = match unsafe { *(Self::CURSOR_ADDR as *const Option<NonZeroUsize>) } {
            // set initial cursor
            None => self.const_heap_end,
            Some(c) => c.get(),
        };
        let new_alloc = alloc_result(cursor, layout);
        match new_alloc {
            None => null_mut(),
            Some(new_alloc) => {
                // deref of this ptr is most likely UB
                unsafe {
                    *(Self::CURSOR_ADDR as *mut Option<NonZeroUsize>) = Some(new_alloc);
                }
                new_alloc.get() as *mut u8
            }
        }
    }

    #[inline]
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // I'm a bump allocator, I don't free.
    }
}

impl<const HEAP_LENGTH: usize> Default for Allogator<HEAP_LENGTH> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
