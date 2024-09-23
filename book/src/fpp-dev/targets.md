# Supported Targets

Kona seeks to support all FPVM targets that LLVM and `rustc` can offer introductory support for. Below is a matrix of features that Kona offers
for each FPVM target:

| Target                 | Build Pipeline | IO  | malloc |
| ---------------------- | -------------- | --- | ------ |
| `cannon` & `cannon-rs` | ✅             | ✅  | ✅     |
| `asterisc`             | ✅             | ✅  | ✅     |

If there is a feature that you would like to see supported, please [open an issue][new-issue] or [consider contributing][contributing]!

## Asterisc (RISC-V)

Asterisc is based off of the `rv64gc` target architecture, which defines the following extensions:

- `RV32I` support - 32 bit base instruction set
  - `FENCE`, `ECALL`, `EBREAK` are hardwired to implement a minimal subset of systemcalls of the linux kernel
    - Work in progress. All syscalls used by the Golang `risc64` runtime.
- `RV64I` support
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

[asterisc-syscalls]: https://github.com/ethereum-optimism/asterisc/blob/master/docs/golang.md#linux-syscalls-used-by-go

## Cannon (MIPS32r2)

Cannon is based off of the `mips32r2` target architecture, specified in [_MIPS32™ Architecture For Programmers Volume III: The MIPS32™ Privileged Resource Architecture_](https://www.cs.cornell.edu/courses/cs3410/2013sp/MIPS_Vol3.pdf)

### Syscalls

Syscalls supported by `cannon` can be found within the `cannon` specification [here][cannon-syscalls].

[cannon-syscalls]: https://specs.optimism.io/fault-proof/cannon-fault-proof-vm.html#syscalls

{{#include ../links.md}}
