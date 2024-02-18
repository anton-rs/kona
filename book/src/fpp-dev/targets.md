# Supported Targets

Kona seeks to support all FPVM targets that LLVM and `rustc` can offer introductory support for. Below is a matrix of features that Kona offers
for each FPVM target:

| Target                 | Build Pipeline | IO  | malloc | Program Stages |
| ---------------------- | -------------- | --- | ------ | -------------- |
| `cannon` & `cannon-rs` | ✅             | ✅  | ✅     | ❌             |
| `asterisc`             | ✅             | ✅  | ✅     | ❌             |

If there is a feature that you would like to see supported, please [open an issue][new-issue] or [consider contributing][contributing]!

## Cannon (MIPS32r2)

Cannon is based off of the `mips32r2` target architecture, supporting 55 instructions:

| Category             | Instruction | Description                               |
| -------------------- | ----------- | ----------------------------------------- |
| `Arithmetic`         | `addi`      | Add immediate (with sign-extension).      |
| `Arithmetic`         | `addiu`     | Add immediate unsigned (no overflow).     |
| `Arithmetic`         | `addu`      | Add unsigned (no overflow).               |
| `Logical`            | `and`       | Bitwise AND.                              |
| `Logical`            | `andi`      | Bitwise AND immediate.                    |
| `Branch`             | `b`         | Unconditional branch.                     |
| `Conditional Branch` | `beq`       | Branch on equal.                          |
| `Conditional Branch` | `beqz`      | Branch if equal to zero.                  |
| `Conditional Branch` | `bgez`      | Branch on greater than or equal to zero.  |
| `Conditional Branch` | `bgtz`      | Branch on greater than zero.              |
| `Conditional Branch` | `blez`      | Branch on less than or equal to zero.     |
| `Conditional Branch` | `bltz`      | Branch on less than zero.                 |
| `Conditional Branch` | `bne`       | Branch on not equal.                      |
| `Conditional Branch` | `bnez`      | Branch if not equal to zero.              |
| `Logical`            | `clz`       | Count leading zeros.                      |
| `Arithmetic`         | `divu`      | Divide unsigned.                          |
| `Unconditional Jump` | `j`         | Jump.                                     |
| `Unconditional Jump` | `jal`       | Jump and link.                            |
| `Unconditional Jump` | `jalr`      | Jump and link register.                   |
| `Unconditional Jump` | `jr`        | Jump register.                            |
| `Data Transfer`      | `lb`        | Load byte.                                |
| `Data Transfer`      | `lbu`       | Load byte unsigned.                       |
| `Data Transfer`      | `lui`       | Load upper immediate.                     |
| `Data Transfer`      | `lw`        | Load word.                                |
| `Data Transfer`      | `lwr`       | Load word right.                          |
| `Data Transfer`      | `mfhi`      | Move from HI register.                    |
| `Data Transfer`      | `mflo`      | Move from LO register.                    |
| `Data Transfer`      | `move`      | Move between registers.                   |
| `Data Transfer`      | `movn`      | Move conditional on not zero.             |
| `Data Transfer`      | `movz`      | Move conditional on zero.                 |
| `Data Transfer`      | `mtlo`      | Move to LO register.                      |
| `Arithmetic`         | `mul`       | Multiply (to produce a word result).      |
| `Arithmetic`         | `multu`     | Multiply unsigned.                        |
| `Arithmetic`         | `negu`      | Negate unsigned.                          |
| `No Op`              | `nop`       | No operation.                             |
| `Logical`            | `not`       | Bitwise NOT (pseudo-instruction in MIPS). |
| `Logical`            | `or`        | Bitwise OR.                               |
| `Logical`            | `ori`       | Bitwise OR immediate.                     |
| `Data Transfer`      | `sb`        | Store byte.                               |
| `Logical`            | `sll`       | Shift left logical.                       |
| `Logical`            | `sllv`      | Shift left logical variable.              |
| `Comparison`         | `slt`       | Set on less than (signed).                |
| `Comparison`         | `slti`      | Set on less than immediate.               |
| `Comparison`         | `sltiu`     | Set on less than immediate unsigned.      |
| `Comparison`         | `sltu`      | Set on less than unsigned.                |
| `Logical`            | `sra`       | Shift right arithmetic.                   |
| `Logical`            | `srl`       | Shift right logical.                      |
| `Logical`            | `srlv`      | Shift right logical variable.             |
| `Arithmetic`         | `subu`      | Subtract unsigned.                        |
| `Data Transfer`      | `sw`        | Store word.                               |
| `Data Transfer`      | `swr`       | Store word right.                         |
| `Serialization`      | `sync`      | Synchronize shared memory.                |
| `System Calls`       | `syscall`   | System call.                              |
| `Logical`            | `xor`       | Bitwise XOR.                              |
| `Logical`            | `xori`      | Bitwise XOR immediate.                    |

### Syscalls

| \$v0 | system call | \$a0            | \$a1       | \$a2         | Effect                                                                                                               |
| ---- | ----------- | --------------- | ---------- | ------------ | -------------------------------------------------------------------------------------------------------------------- |
| 4090 | mmap        | uint32 addr     | uint32 len | 🚫           | Allocates a page from the heap. See [heap](#heap) for details.                                                       |
| 4045 | brk         | 🚫              | 🚫         | 🚫           | Returns a fixed address for the program break at `0x40000000`                                                        |
| 4120 | clone       | 🚫              | 🚫         | 🚫           | Returns 1                                                                                                            |
| 4246 | exit_group  | uint8 exit_code | 🚫         | 🚫           | Sets the Exited and ExitCode states to `true` and `$a0` respectively.                                                |
| 4003 | read        | uint32 fd       | char \*buf | uint32 count | Similar behavior as Linux/MIPS with support for unaligned reads. See [I/O](#io) for more details.                    |
| 4004 | write       | uint32 fd       | char \*buf | uint32 count | Similar behavior as Linux/MIPS with support for unaligned writes. See [I/O](#io) for more details.                   |
| 4055 | fcntl       | uint32 fd       | int32 cmd  | 🚫           | Similar behavior as Linux/MIPS. Only the `F_GETFL` (3) cmd is supported. Sets errno to `0x16` for all other commands |

For all of the above syscalls, an error is indicated by setting the return
register (`$v0`) to `0xFFFFFFFF` (-1) and `errno` (`$a3`) is set accordingly.
The VM must not modify any register other than `$v0` and `$a3` during syscall handling.
For unsupported syscalls, the VM must do nothing except to zero out the syscall return (`$v0`)
and errno (`$a3`) registers.

Note that the above syscalls have identical syscall numbers and ABIs as Linux/MIPS.

## Asterisc (RISC-V)

Asterisc is based off of the `rv64gc` target architecture, which defines the following extensions:

- `RV32I` support - 32 bit base instruction set
  - `FENCE`, `ECALL`, `EBREAK` are hardwired to implement a minimal subset of systemcalls of the linux kernel
    - Work in progress. All syscalls used by the Golang `risc64` runtime.
- `RV64I` support
- `RV64C`: Compressed instructions
- `RV32M`+`RV64M`: Multiplication support
- `RV32A`+`RV64A`: Atomics support
- `RV{32,64}{D,F,Q}`: no-op: No floating points support (since no IEEE754 determinism with rounding modes etc., nor worth the complexity)
- `Zifencei`: `FENCE.I` no-op: No need for `FENCE.I`
- `Zicsr`: no-op: some support for Control-and-status registers may come later though.
- `Ztso`: no-op: no need for Total Store Ordering
- other: revert with error code on unrecognized instructions

`asterisc` supports a plethora of syscalls, documented [in the repository][asterisc-syscalls]. `kona` offers an interface for
programs to directly invoke a select few syscalls:

1. `EXIT` - Terminate the process with the provided exit code.
1. `WRITE` - Write the passed buffer to the passed file descriptor.
1. `READ` - Read the specified number of bytes from the passed file descriptor.

[asterisc-syscalls]: https://github.com/ethereum-optimism/asterisc

{{#include ../links.md}}
