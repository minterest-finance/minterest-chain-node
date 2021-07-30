#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

_term() { 
  echo "Caught SIGTERM signal!" 
  kill -TERM "$child" 2>/dev/null
}
trap _term SIGTERM

# --------------------

NODE=../target/release/minterest
PARA_ARGS=(--tmp --parachain-id 2000)
COLLATOR_ARGS=(--collator)
RELAY_ARGS=(--chain rococo-local.json)

# Collator1
$NODE \
  --alice --port 40335 --ws-port 9946 ${PARA_ARGS[@]} ${COLLATOR_ARGS[@]} \
  -- --alice --port 30335 ${RELAY_ARGS[@]} &

# Collator2
$NODE \
  --bob --port 40336 --ws-port 9947 ${PARA_ARGS[@]} ${COLLATOR_ARGS[@]} \
  -- --bob --port 30336 ${RELAY_ARGS[@]} > /dev/null 2>&1 &

# Parachain Full Node 1
$NODE \
  --port 40337 --ws-port 9948 ${PARA_ARGS[@]} \
  -- --port 30337 ${RELAY_ARGS[@]} > /dev/null 2>&1 &

# --------------------

child=$! 
wait "$child"