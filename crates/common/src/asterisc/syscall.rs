//! Unsafe system call interface for the `riscv64` target architecture.
//!
//! List of RISC-V system calls: <https://jborza.com/post/2021-05-11-riscv-linux-syscalls/>
//!
//! **Registers used for system calls**
//! | Register Number |    Description     |
//! |=================|====================|
//! | %a0             | arg1, return value |
//! | %a1             | arg2               |
//! | %a2             | arg3               |
//! | %a3             | arg4               |
//! | %a4             | arg5               |
//! | %a5             | arg6               |
//! | %a7             | syscall number     |

use crate::types::RegisterSize;
use core::arch::asm;

/// Issues a raw system call with 1 argument. (e.g. exit)
#[inline]
pub(crate) unsafe fn syscall1(syscall_number: RegisterSize, arg1: RegisterSize) -> RegisterSize {
    let mut ret: RegisterSize;
    asm!(
        "ecall",
        in("a7") syscall_number,
        inlateout("a0") arg1 => ret,
        options(nostack, preserves_flags)
    );
    ret
}

/// Issues a raw system call with 3 arguments. (e.g. read, write)
#[inline]
pub(crate) unsafe fn syscall3(
    syscall_number: RegisterSize,
    arg1: RegisterSize,
    arg2: RegisterSize,
    arg3: RegisterSize,
) -> RegisterSize {
    let mut ret: RegisterSize;
    asm!(
        "ecall",
        in("a7") syscall_number,
        inlateout("a0") arg1 => ret,
        in("a1") arg2,
        in("a2") arg3,
        options(nostack, preserves_flags)
    );
    ret
}
