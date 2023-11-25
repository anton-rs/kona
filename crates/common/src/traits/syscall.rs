//! Defines the [KernelIO] trait, which describes the functionality of several
//! system calls inside of the FPVM kernel. This trait is to be implemented by the
//! `client` program, and then used by the `kernel` to perform IO operations.

use anyhow::Result;
use num::Unsigned;

/// The [KernelIO] trait describes the functionality of several system calls inside of
/// the FPVM kernel. Commonly, FPVMs delegate IO operations to custom file descriptors in
/// the `client` program.
///
///
/// The `RS` type parameter is the size of the registers in the VM. On MIPS32 for example,
/// this would be 32, and on RISC-V/64 this would be 64.
///
/// In cases where the set of system calls defined in this trait need to be extended, an
/// additional trait should be created that extends this trait.
pub trait KernelIO<RS: Unsigned> {
    /// Associated type for the file descriptors available.
    type FileDescriptor;

    /// Write the given buffer to the given file descriptor.
    fn write(fd: Self::FileDescriptor, buf: &[u8]) -> Result<RS>;

    /// Read from the given file descriptor into the passed buffer.
    fn read(fd: Self::FileDescriptor, buf: &mut [u8]) -> Result<RS>;

    /// Exit the process with the given exit code. The implementation of this function
    /// should always panic after invoking the `EXIT` syscall.
    fn exit(code: RS) -> !;
}
