#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'


NODE=../target/release/minterest

$NODE export-genesis-state --parachain-id 2000 > minterest-para-genesis-2000
$NODE export-genesis-wasm > minterest-para-wasm