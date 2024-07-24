use crate::{BasicKernelInterface, FileDescriptor};
use anyhow::Result;

/// Concrete implementation of the [`KernelIO`] trait for the `SP1` target architecture.
#[derive(Debug)]
pub struct ZkvmIO;

impl BasicKernelInterface for ZkvmIO {
    fn write(_fd: FileDescriptor, _buf: &[u8]) -> Result<usize> {
        unimplemented!();
    }

    fn read(_fd: FileDescriptor, _buf: &mut [u8]) -> Result<usize> {
        unimplemented!();
    }

    fn exit(_code: usize) -> ! {
        unimplemented!();
    }
}
