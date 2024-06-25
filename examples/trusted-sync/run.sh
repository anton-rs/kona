#!/usr/bin/bash

/usr/local/bin/trusted-sync \
  --l1-rpc-url $L1_RPC_URL \
  --l2-rpc-url $L2_RPC_URL \
  --beacon-url $BEACON_URL \
  --start-l2-block $START_L2_BLOCK \
  --metrics-server-addr $METRICS_SERVER_ADDR \
  --metrics-server-port $METRICS_SERVER_PORT \
  -vvv
