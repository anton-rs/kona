use crate::{cannon::syscall, BasicKernelInterface, FileDescriptor, RegisterSize};
use anyhow::{anyhow, Result};

/// Concrete implementation of the [BasicKernelInterface] trait for the `MIPS32rel1` target architecture. Exposes a safe
/// interface for performing IO operations within the FPVM kernel.
#[derive(Debug)]
pub struct CannonIO;

/// Relevant system call numbers for the `MIPS32rel1` target architecture.
///
/// See [Cannon System Call Specification](https://github.com/ethereum-optimism/optimism/blob/develop/specs/cannon-fault-proof-vm.md#syscalls)
///
/// **Note**: This is not an exhaustive list of system calls available to the `client` program,
/// only the ones necessary for the [BasicKernelInterface] trait implementation. If an extension trait for
/// the [BasicKernelInterface] trait is created for the `Cannon` kernel, this list should be extended
/// accordingly.
#[repr(u32)]
pub(crate) enum SyscallNumber {
    /// Sets the Exited and ExitCode states to true and $a0 respectively.
    Exit = 4246,
    /// Similar behavior as Linux/MIPS with support for unaligned reads.
    Read = 4003,
    /// Similar behavior as Linux/MIPS with support for unaligned writes.
    Write = 4004,
}

impl BasicKernelInterface for CannonIO {
    fn write(fd: FileDescriptor, buf: &[u8]) -> Result<RegisterSize> {
        unsafe {
            syscall::syscall3(
                SyscallNumber::Write as u32,
                fd.into(),
                buf.as_ptr() as u32,
                buf.len() as u32,
            )
            .map_err(|e| anyhow!("Syscall Error: {e}"))
        }
    }

    fn read(fd: FileDescriptor, buf: &mut [u8]) -> Result<RegisterSize> {
        unsafe {
            syscall::syscall3(
                SyscallNumber::Read as u32,
                fd.into(),
                buf.as_ptr() as u32,
                buf.len() as u32,
            )
            .map_err(|e| anyhow!("Syscall Error: {e}"))
        }
    }

    fn exit(code: RegisterSize) -> ! {
        unsafe {
            syscall::syscall1(SyscallNumber::Exit as RegisterSize, code);
            panic!()
        }
    }
}
