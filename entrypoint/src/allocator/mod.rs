mod allogator;
mod no;

pub use allogator::*;
pub use no::*;

/// Start address of the memory region used for program heap.
pub const HEAP_START_ADDRESS: usize = 0x300000000;

/// Length of the heap memory region used for program heap.
pub const HEAP_LENGTH: usize = 32 * 1024;

/// Default global allocator.
///
/// This macro sets up a default global allocator that uses a bump allocator to allocate memory.
#[macro_export]
macro_rules! default_allocator {
    () => {
        #[cfg(target_os = "solana")]
        #[global_allocator]
        static A: $crate::allocator::Allogator = $crate::allocator::Allogator::new();
    };
}

/// A global allocator that does not allocate memory.
///
/// This macro sets up a global allocator that denies all allocations. This is useful when the
/// program does not need to allocate memory $mdash; the program will panic if it tries to
/// allocate memory.
#[macro_export]
macro_rules! no_allocator {
    () => {
        #[cfg(target_os = "solana")]
        #[global_allocator]
        static A: $crate::allocator::NoAllocator = $crate::allocator::NoAllocator;
    };
}
