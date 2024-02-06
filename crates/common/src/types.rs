//! This module contains the local types for the `kona-common` crate.

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "mips")] {
        /// The size of the `mips32` target architecture's registers.
        pub type RegisterSize = u32;
    } else if #[cfg(target_arch = "riscv64")] {
        /// The size of the `riscv64` target architecture's registers.
        pub type RegisterSize = u64;
    } else {
        /// The size of the native target architecture's registers.
        pub type RegisterSize = u64;
    }
}
