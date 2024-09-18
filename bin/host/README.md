# `kona-host`

kona-host is a CLI application that runs the [pre-image server][p-server] and [client program][client-program].

## Modes

| Mode     | Description                                                                                                                                                                             |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `server` | Starts with the preimage server only, expecting the client program to have been invoked by the host process. This mode is intended for use by the FPVM when running the client program. |
| `native` | Starts both the preimage oracle and client program in a native process. This mode is useful for witness generation as well as testing.                                                  |

## Usage

```
Usage: kona-host [OPTIONS] --l1-head <L1_HEAD> --l2-head <L2_HEAD> --l2-output-root <L2_OUTPUT_ROOT> --l2-claim <L2_CLAIM> --l2-block-number <L2_BLOCK_NUMBER>

Options:
  -v, --v...
          Verbosity level (0-2)
      --l1-head <L1_HEAD>
          Hash of the L1 head block. Derivation stops after this block is processed
      --l2-head <L2_HEAD>
          Hash of the L2 block committed to by `--l2-output-root`
      --l2-output-root <L2_OUTPUT_ROOT>
          Agreed L2 Output Root to start derivation from
      --l2-claim <L2_CLAIM>
          Claimed L2 output root at block # `--l2-block-number` to validate
      --l2-block-number <L2_BLOCK_NUMBER>
          Number of the L2 block that the claim commits to
      --l2-node-address <L2_NODE_ADDRESS>
          Address of L2 JSON-RPC endpoint to use (eth and debug namespace required) [aliases: l2]
      --l1-node-address <L1_NODE_ADDRESS>
          Address of L1 JSON-RPC endpoint to use (eth and debug namespace required) [aliases: l1]
      --l1-beacon-address <L1_BEACON_ADDRESS>
          Address of the L1 Beacon API endpoint to use [aliases: beacon]
      --data-dir <DATA_DIR>
          The Data Directory for preimage data storage. Default uses in-memory storage [aliases: db]
      --exec <EXEC>
          Run the specified client program natively as a separate process detached from the host
      --server
          Run in pre-image server mode without executing any client program. If not provided, the host will run the client program in the host process
      --l2-chain-id <L2_CHAIN_ID>
          The L2 chain ID of a supported chain. If provided, the host will look for the corresponding rollup config in the superchain registry
      --rollup-config-path <ROLLUP_CONFIG_PATH>
          Path to rollup config. If provided, the host will use this config instead of attempting to look up the config in the superchain registry
  -h, --help
          Print help
  -V, --version
          Print version
```

[p-server]: https://specs.optimism.io/fault-proof/index.html#pre-image-oracle
[client-program]: https://specs.optimism.io/fault-proof/index.html#fault-proof-program
