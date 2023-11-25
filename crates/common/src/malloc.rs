//! This module contains an implementation of a basic memory allocator for client programs in
//! running on top of various FPVMs. The allocator is a linked list allocator based on the
//! `dlmalloc` algorithm, which is a well-known and widely used allocator software such
//! as OS Kernels.

#[cfg(not(test))]
use good_memory_allocator::SpinLockedAllocator;

/// The global allocator for the program in the `test` profile uses the standard allocator.
#[cfg(test)]
#[global_allocator]
static ALLOCATOR: std::alloc::System = std::alloc::System;

/// The global allocator for the program in other profiles uses the [SpinLockedAllocator].
#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: SpinLockedAllocator = SpinLockedAllocator::empty();

/// Initialize the [SpinLockedAllocator] with the following parameters:
/// * `heap_start_addr` is the starting address of the heap memory region,
/// * `heap_size` is the size of the heap memory region in bytes.
///
/// # Safety
/// This function is unsafe because the caller must ensure:
/// * The allocator has not already been initialized.
/// * The provided memory region must be valid, non-null, and not used by anything else.
/// * After aligning the start and end addresses, the size of the heap must be > 0, or the function
///   will panic.
#[cfg_attr(test, allow(unused_variables))]
pub unsafe fn init_allocator(heap_start_addr: usize, heap_size: usize) {
    #[cfg(not(test))]
    ALLOCATOR.init(heap_start_addr, heap_size)
}

/// Initialize heap memory for the `client` program with
#[macro_export]
macro_rules! init_heap {
    ($size:expr) => {{
        use kona_common::malloc::init_allocator;

        static mut HEAP: [u8; $size] = [0u8; $size];
        unsafe { init_allocator(HEAP.as_ptr() as usize, $size) }
    }};
}
