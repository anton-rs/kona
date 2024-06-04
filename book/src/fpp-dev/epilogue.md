# Epilogue

The epilogue stage of the program is intended to perform the final validation on the outputs from the
[execution phase](./execution.md). In most programs, this entails comparing the outputs of the execution phase
to portions of the bootstrap data made available during the [prologue phase](./prologue.md).

Generally, this phase should consist almost entirely of validation steps.

## Example

In the `kona-client` program, the prologue phase only contains two directives:

1. Validate that the L2 safe chain could be produced at the claimed L2 block height.
1. The constructed output root is equivalent to the claimed [L2 output root][l2-output-root].

{{#include ../links.md}}
