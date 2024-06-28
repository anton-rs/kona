#!/usr/bin/bash

START="${START_L2_BLOCK:-0}"
METRICS="${METRICS_URL:-127.0.0.1:9000}"

/usr/local/bin/trusted-sync \
  --l1-rpc-url $L1_RPC_URL \
  --l2-rpc-url $L2_RPC_URL \
  --beacon-url $BEACON_URL \
  --metrics-url $METRICS \
  --start-l2-block $START \
  -vvv
