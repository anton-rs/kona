# `kona-host`

kona-host is a CLI application that runs the [pre-image server][p-server] and [client program][client-program].

## Modes

| Mode     | Description                                                                                                                                                                             |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `server` | Starts with the preimage server only, expecting the client program to have been invoked by the host process. This mode is intended for use by the FPVM when running the client program. |
| `native` | Starts both the preimage oracle and client program in a native process. This mode is useful for witness generation as well as testing.                                                  |

## Usage

```txt
kona-host is a CLI application that runs the Kona pre-image server and client program. The host
can run in two modes: server mode and native mode. In server mode, the host runs the pre-image
server and waits for the client program in the parent process to request pre-images. In native
mode, the host runs the client program in a separate thread with the pre-image server in the
primary thread.


Usage: kona-host [OPTIONS] --l1-head <L1_HEAD> --agreed-l2-head-hash <AGREED_L2_HEAD_HASH> --agreed-l2-output-root <AGREED_L2_OUTPUT_ROOT> --claimed-l2-output-root <CLAIMED_L2_OUTPUT_ROOT> --claimed-l2-block-number <CLAIMED_L2_BLOCK_NUMBER>

Options:
  -v, --v...
          Verbosity level (0-2)
      --l1-head <L1_HEAD>
          Hash of the L1 head block. Derivation stops after this block is processed [env: L1_HEAD=]
      --agreed-l2-head-hash <AGREED_L2_HEAD_HASH>
          Hash of the agreed upon safe L2 block committed to by `--agreed-l2-output-root` [env: AGREED_L2_HEAD_HASH=] [aliases: l2-head]
      --agreed-l2-output-root <AGREED_L2_OUTPUT_ROOT>
          Agreed safe L2 Output Root to start derivation from [env: AGREED_L2_OUTPUT_ROOT=] [aliases: l2-output-root]
      --claimed-l2-output-root <CLAIMED_L2_OUTPUT_ROOT>
          Claimed L2 output root at block # `--claimed-l2-block-number` to validate [env: CLAIMED_L2_OUTPUT_ROOT=] [aliases: l2-claim]
      --claimed-l2-block-number <CLAIMED_L2_BLOCK_NUMBER>
          Number of the L2 block that the claimed output root commits to [env: CLAIMED_L2_BLOCK_NUMBER=] [aliases: l2-block-number]
      --l2-node-address <L2_NODE_ADDRESS>
          Address of L2 JSON-RPC endpoint to use (eth and debug namespace required) [env: L2_NODE_ADDRESS=] [aliases: l2]
      --l1-node-address <L1_NODE_ADDRESS>
          Address of L1 JSON-RPC endpoint to use (eth and debug namespace required) [env: L1_NODE_ADDRESS=] [aliases: l1]
      --l1-beacon-address <L1_BEACON_ADDRESS>
          Address of the L1 Beacon API endpoint to use [env: L1_BEACON_ADDRESS=] [aliases: beacon]
      --data-dir <DATA_DIR>
          The Data Directory for preimage data storage. Optional if running in online mode, required if running in offline mode [env: DATA_DIR=] [aliases: db]
      --native
          Run the specified client program natively
      --server
          Run in pre-image server mode without executing any client program. If not provided, the host will run the client program in the host process
      --l2-chain-id <L2_CHAIN_ID>
          The L2 chain ID of a supported chain. If provided, the host will look for the corresponding rollup config in the superchain registry [env: L2_CHAIN_ID=]
      --rollup-config-path <ROLLUP_CONFIG_PATH>
          Path to rollup config. If provided, the host will use this config instead of attempting to look up the config in the superchain registry [env: ROLLUP_CONFIG_PATH=]
  -h, --help
          Print help
  -V, --version
          Print version
```

[p-server]: https://specs.optimism.io/fault-proof/index.html#pre-image-oracle
[client-program]: https://specs.optimism.io/fault-proof/index.html#fault-proof-program
