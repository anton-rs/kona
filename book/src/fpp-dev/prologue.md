# Prologue

The prologue stage of the program is commonly responsible for bootstrapping the program with inputs from an external
source, pulled in through the [Host <-> Client communication](./env.md#host---client-communication) implementation.

As a rule of thumb, the prologue implementation should be kept minimal, and should not do much more than establish
the inputs for the [execution phase](./execution.md).

## Example

As an example, the prologue stage of the `kona-client` program runs through several steps:

1. Pull in the boot information over the [Preimage Oracle ABI][preimage-specs], containing:
   - The L1 head hash containing all data required to reproduce the L2 safe chain at the claimed block height.
   - The latest finalized [L2 output root][l2-output-root].
   - The [L2 output root][l2-output-root] claim.
   - The block number of the [L2 output root][l2-output-root] claim.
   - The L2 chain ID.
1. Pull in the `RollupConfig` and `L2ChainConfig` corresponding to the passed L2 chain ID.
1. Validate these values.
1. Pass the boot information to the execution phase.

{{#include ../links.md}}
