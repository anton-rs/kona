# `kona-optimism`

This binary contains the client program for executing the Optimism rollup state transition.

## Modes

The `kona-optimism` program supports several different modes, each with a separate purpose:

| Name     | Description                                                                                                                                                                                                                |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `online` | Directly fetches external data from trusted providers. To be invoked without the `host` program on native hardware.                                                                                                        |
| `fault`  | Fetches in external data over the wire through the [`PreimageOracle` ABI][preimage-oracle-abi], supported by the `kona-host` program. Can run on native hardware or one of the supported [Fault Proof VM][fpvm] soft-CPUs. |

[preimage-oracle-abi]: https://specs.optimism.io/experimental/fault-proof/index.html#pre-image-oracle
[fpvm]: https://static.optimism.io/kona/fpp-dev/targets.html
