# Execution

The execution phase of the program is commonly the heaviest portion of the fault proof program, where the computation
that is being verified is performed.

This phase consumes the outputs of the [prologue phase](./prologue.md), and performs the bulk of the verifiable
computation. After execution has concluded, the outputs are passed along to the [epilogue phase](./epilogue.md) for
final verification.

## Example

At a high-level, in the `kona-client` program, the execution phase:

1. Derives the inputs to the L2 derivation pipeline by unrolling the L1 head hash fetched in the epilogue.
1. Passes the inputs to the L2 derivation pipeline, producing the L2 execution payloads required to reproduce
   the L2 safe chain at the claimed height.
1. Executes the payloads produced by the L2 derivation pipeline, producing the [L2 output root][l2-output-root] at the
   L2 claim height.

{{#include ../links.md}}
