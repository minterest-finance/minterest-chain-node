#!/usr/bin/env bash

# https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/source-based-code-coverage.html

# Requirements:
# rustup component add llvm-tools-preview
# cargo install rustfilt cargo-binutils
# sudo apt install jq

# Usage: ./covsum.sh or ./covsum.sh > result.json

# How it works. It runs unit tests for each pallet and takes coverage only from lib.rs.

# See information about lines and regions coverage
# https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/source-based-code-coverage.html#interpreting-reports

# Add pallet-name here to include in coverage report
Pallets=('risk-manager' 'controller' 'dex' \
    'liquidation-pools' 'liquidity-pools' \
    'minterest-model' 'mnt-token' 'prices')

set -e

summary='[]'

for pallet_name in "${Pallets[@]}"; do
    # Go to pallet, compile tests and get binary path
    cd pallets/$pallet_name
    bin_path=$(RUSTFLAGS="-Zinstrument-coverage" cargo test --no-run --message-format=json \
        | jq -r "select(.profile.test == true) | .filenames[]")
    cd ../..

    # Run binary to generate profraw file
    LLVM_PROFILE_FILE="formatjson5.profraw" $bin_path &> /dev/null

    # Generate summary
    cargo profdata -- merge -sparse  formatjson5.profraw -o json5format.profdata
    cov_sum=$(cargo cov -- export -Xdemangler=rustfilt --format='text' \
        --ignore-filename-regex='/.cargo/registry|toolchains/nightly|mock.rs|tests.rs' \
        --instr-profile=json5format.profdata  -summary-only  --object $bin_path)
    cov_result=$(echo $cov_sum | jq ".data[].files[] | select(.filename==\"pallets/$pallet_name/src/lib.rs\").summary |
            {pallet: \"$pallet_name\",
            lines_coverage: .lines.percent,
            regions_coverage: .regions.percent,
            functions_coverage: .functions.percent}")
    summary=$(echo $summary | jq ".[length] |= . + $cov_result")
done

echo $summary | jq

rm formatjson5.profraw
rm json5format.profdata
# This artifact occasionaly apeears.
rm default.profraw 2> /dev/null || true
