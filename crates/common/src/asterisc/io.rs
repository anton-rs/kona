use crate::{
    asterisc::syscall, io::FileDescriptor, traits::BasicKernelInterface, types::RegisterSize,
};
use anyhow::Result;

/// Concrete implementation of the [`KernelIO`] trait for the `riscv64` target architecture.
#[derive(Debug)]
pub struct AsteriscIO;

/// Relevant system call numbers for the `riscv64` target architecture.
///
/// See https://jborza.com/post/2021-05-11-riscv-linux-syscalls/
///
/// **Note**: This is not an exhaustive list of system calls available to the `client` program,
/// only the ones necessary for the [BasicKernelInterface] trait implementation. If an extension trait for
/// the [BasicKernelInterface] trait is created for the `asterisc` kernel, this list should be extended
/// accordingly.
#[repr(u32)]
pub(crate) enum SyscallNumber {
    /// Sets the Exited and ExitCode states to true and $a0 respectively.
    Exit = 93,
    /// Similar behavior as Linux with support for unaligned reads.
    Read = 63,
    /// Similar behavior as Linux with support for unaligned writes.
    Write = 64,
}

impl BasicKernelInterface for AsteriscIO {
    fn write(fd: FileDescriptor, buf: &[u8]) -> Result<RegisterSize> {
        unsafe {
            Ok(syscall::syscall3(
                SyscallNumber::Write as usize,
                fd as usize,
                buf.as_ptr() as usize,
                buf.len() as usize,
            ) as RegisterSize)
        }
    }

    fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<RegisterSize> {
        unsafe {
            Ok(syscall::syscall3(
                SyscallNumber::Read as usize,
                fd as usize,
                buf.as_ptr() as usize,
                buf.len() as usize,
            ) as RegisterSize)
        }
    }

    fn exit(code: RegisterSize) -> ! {
        unsafe {
            syscall::syscall1(SyscallNumber::Exit as usize, code as usize);
            panic!()
        }
    }
}
