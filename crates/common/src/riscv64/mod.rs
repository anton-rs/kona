//! This module contains raw syscall bindings for the `riscv64gc` target architecture, as well as a
//! high-level implementation of the [crate::BasicKernelInterface] trait for the `asterisc` kernel.

pub(crate) mod io;
mod syscall;
