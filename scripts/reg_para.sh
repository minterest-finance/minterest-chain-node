#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

GENESIS="$(cat minterest-para-genesis-2000)"
WASM="$(cat minterest-para-wasm)"
echo "2000 {\"genesisHead\":\"$GENESIS\",\"validationCode\":\"$WASM\",\"parachain\":true}" > /tmp/reg_paras_params

polkadot-js-api \
  --ws ws://127.0.0.1:9944 --params /tmp/reg_paras_params \
  --sudo --seed "//Alice" \
  tx.parasSudoWrapper.sudoScheduleParaInitialize