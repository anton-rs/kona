//! This module contains an implementation of a basic memory allocator for client programs in
//! running on top of various FPVMs. The allocator is a linked list allocator based on the
//! `dlmalloc` algorithm, which is a well-known and widely used allocator software such
//! as OS Kernels.

/// The global allocator for the program in FPVM environments.
#[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
pub mod global_allocator {
    use good_memory_allocator::SpinLockedAllocator;

    /// The global allocator for the program in other profiles uses the [SpinLockedAllocator].
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
    pub unsafe fn init_allocator(heap_start_addr: usize, heap_size: usize) {
        ALLOCATOR.init(heap_start_addr, heap_size)
    }
}

/// Initialize heap memory for the `client` program with the given size.
///
/// # Safety
/// See [init_allocator] safety comment.
#[macro_export]
macro_rules! alloc_heap {
    ($size:expr) => {{
        #[cfg(any(target_arch = "mips", target_arch = "riscv64"))]
        {
            use kona_common::malloc::inner::init_allocator;

            static mut HEAP: [u8; $size] = [0u8; $size];
            unsafe { init_allocator(HEAP.as_ptr() as usize, $size) }
        }
    }};
}
