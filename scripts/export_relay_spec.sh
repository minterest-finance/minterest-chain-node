#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'


NODE=../../polkadot/target/release/polkadot
$NODE build-spec --chain rococo-local --disable-default-bootnode --raw > rococo-local.json