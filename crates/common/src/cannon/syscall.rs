//! Taken from the syscalls crate <https://github.com/jasonwhite/syscalls>
//!
//! MIPS has the following registers:
//!
//! | Symbolic Name | Number          | Usage                          |
//! | ============= | =============== | ============================== |
//! | zero          | 0               | Constant 0.                    |
//! | at            | 1               | Reserved for the assembler.    |
//! | v0 - v1       | 2 - 3           | Result Registers.              |
//! | a0 - a3       | 4 - 7           | Argument Registers 1 ·· · 4.   |
//! | t0 - t9       | 8 - 15, 24 - 25 | Temporary Registers 0 · · · 9. |
//! | s0 - s7       | 16 - 23         | Saved Registers 0 ·· · 7.      |
//! | k0 - k1       | 26 - 27         | Kernel Registers 0 ·· · 1.     |
//! | gp            | 28              | Global Data Pointer.           |
//! | sp            | 29              | Stack Pointer.                 |
//! | fp            | 30              | Frame Pointer.                 |
//! | ra            | 31              | Return Address.                |
//!
//! The following registers are used for args 1-6:
//!
//! arg1: %a0 ($4)
//! arg2: %a1 ($5)
//! arg3: %a2 ($6)
//! arg4: %a3 ($7)
//! arg5: (Passed via user stack)
//! arg6: (Passed via user stack)
//! arg7: (Passed via user stack)
//!
//! %v0 is the syscall number.
//! %v0 is the return value.
//! %v1 is the error code
//! %a3 is a boolean indicating that an error occurred.
//!
//!
//! All temporary registers are clobbered (8-15, 24-25).

use crate::types::RegisterSize;
use core::arch::asm;

/// Issues a raw system call with 1 argument. (e.g. exit)
#[inline]
pub(crate) unsafe fn syscall1(n: RegisterSize, arg1: RegisterSize) -> RegisterSize {
    let mut err: RegisterSize;
    let mut ret: RegisterSize;
    asm!(
        "syscall",
        inlateout("$2") n => ret,
        lateout("$7") err,
        in("$4") arg1,
        // Clobber all temporary registers
        lateout("$8") _,
        lateout("$9") _,
        lateout("$10") _,
        lateout("$11") _,
        lateout("$12") _,
        lateout("$13") _,
        lateout("$14") _,
        lateout("$15") _,
        lateout("$24") _,
        lateout("$25") _,
        options(nostack, preserves_flags)
    );

    (err == 0)
        .then_some(ret)
        .unwrap_or_else(|| ret.wrapping_neg())
}

/// Issues a raw system call with 3 arguments. (e.g. read, write)
#[inline]
pub(crate) unsafe fn syscall3(
    n: RegisterSize,
    arg1: RegisterSize,
    arg2: RegisterSize,
    arg3: RegisterSize,
) -> Result<RegisterSize, i32> {
    let mut err: RegisterSize;
    let mut ret: RegisterSize;
    asm!(
        "syscall",
        inlateout("$2") n => ret,
        lateout("$7") err,
        in("$4") arg1,
        in("$5") arg2,
        in("$6") arg3,
        // Clobber all temporary registers
        lateout("$8") _,
        lateout("$9") _,
        lateout("$10") _,
        lateout("$11") _,
        lateout("$12") _,
        lateout("$13") _,
        lateout("$14") _,
        lateout("$15") _,
        lateout("$24") _,
        lateout("$25") _,
        options(nostack, preserves_flags)
    );

    let value = (err == 0)
        .then_some(ret)
        .unwrap_or_else(|| ret.wrapping_neg());

    (value <= -4096isize as RegisterSize)
        .then_some(value)
        .ok_or_else(|| {
            // Truncation of the error value is guaranteed to never occur due to
            // the above check. This is the same check that musl uses:
            // https://git.musl-libc.org/cgit/musl/tree/src/internal/syscall_ret.c?h=v1.1.15
            -(value as i32)
        })
}
