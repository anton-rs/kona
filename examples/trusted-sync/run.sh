#!/usr/bin/bash

START="${START_L2_BLOCK:-0}"
METRICS="${METRICS_URL:-0.0.0.0:9000}"

# If the `START_BLOCKS_FROM_TIP` environment variable is set, we will start syncing from the tip of the chain.
if [ -n "$START_BLOCKS_FROM_TIP" ]; then
  /usr/local/bin/trusted-sync \
    --l1-rpc-url $L1_RPC_URL \
    --l2-rpc-url $L2_RPC_URL \
    --beacon-url $BEACON_URL \
    --metrics-url $METRICS \
    --start-blocks-from-tip $START_BLOCKS_FROM_TIP \
    -vvv
else
  /usr/local/bin/trusted-sync \
    --l1-rpc-url $L1_RPC_URL \
    --l2-rpc-url $L2_RPC_URL \
    --beacon-url $BEACON_URL \
    --metrics-url $METRICS \
    --start-l2-block $START \
    -vvv
fi
