//! Defines the [BasicKernelInterface] trait, which describes the functionality of several system calls inside of
//! the FPVM kernel.

use crate::{io::FileDescriptor, types::RegisterSize};
use anyhow::Result;

/// The [BasicKernelInterface] trait describes the functionality of several core system calls inside of
/// the FPVM kernel. Commonly, FPVMs delegate IO operations to custom file descriptors in the `client` program. It is
/// a safe wrapper around the raw system calls available to the `client` program.
///
///
/// The `RS` type parameter is the size of the registers in the VM. On MIPS32 for example, this would be 32, and on
/// RISC-V/64 this would be 64.
///
/// In cases where the set of system calls defined in this trait need to be extended, an additional trait should be
/// created that extends this trait.
pub trait BasicKernelInterface {
    /// Write the given buffer to the given file descriptor.
    fn write(fd: FileDescriptor, buf: &[u8]) -> Result<RegisterSize>;

    /// Read from the given file descriptor into the passed buffer.
    fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<RegisterSize>;

    /// Exit the process with the given exit code. The implementation of this function
    /// should always panic after invoking the `EXIT` syscall.
    fn exit(code: RegisterSize) -> !;
}
