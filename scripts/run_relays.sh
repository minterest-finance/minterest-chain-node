#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

_term() { 
  echo "Caught SIGTERM signal!" 
  kill -TERM "$child" 2>/dev/null
}
trap _term SIGTERM

# --------------------

NODE=../../polkadot/target/release/polkadot
RELAY_ARGS=(--tmp --chain rococo-local.json --rpc-methods Unsafe)

$NODE ${RELAY_ARGS[@]} --alice &
$NODE ${RELAY_ARGS[@]} --bob --port 30434 > /dev/null 2>&1 &
$NODE ${RELAY_ARGS[@]} --charlie --port 30435 > /dev/null 2>&1 &
$NODE ${RELAY_ARGS[@]} --dave --port 30436 > /dev/null 2>&1 &

# --------------------

child=$! 
wait "$child"