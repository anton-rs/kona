# `kona-common`

This library offers utilities for developing verifiable `client` executables that may run on top of Fault Proof Virtual
Machine targets.

- The `alloc_heap` macro allows for statically allocating a heap of a certain size, and all `client` programs will need
  to run it if they require heap allocation. The `alloc` crate can be used for programs targeting any FPVM, but is
  optional.
- The `io` module provides a high-level safe interface over the `read`, `write`, and `exit` syscalls on all available
  FPVM targets. While the FPVMs support a larger set of Linux syscalls, this crate looks to support the bare-minimum
  required for `client` programs to communicate back and forth with the host and exit properly. If a consumer of the
  library would like to extend the functionality of the `ClientIO`, an extension trait can be made for the `ClientIO`
  type in the `io` module.
