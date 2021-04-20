#!/usr/bin/env bash

# https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/source-based-code-coverage.html

# Requirements:
# cargo install rustfilt
# cargo install cargo-binutils
# sudo apt install jq

# Usage: ./covgen.sh <pallet_name>

# Output result path: ./covgen/<pallet_name>/index.html

set -e

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters. Example: ./covgen.sh <pallet_name>"
    exit 1
fi

cd pallets/$1
bin_path=$(RUSTFLAGS="-Zinstrument-coverage" cargo test --no-run --message-format=json \
    | jq -r "select(.profile.test == true) | .filenames[]")
cd ../..

LLVM_PROFILE_FILE="formatjson5.profraw" $bin_path
cargo profdata -- merge -sparse  formatjson5.profraw -o json5format.profdata

cargo cov -- show -Xdemangler=rustfilt $bin_path \
    --ignore-filename-regex='/.cargo/registry|toolchains/nightly|mock.rs|tests.rs' \
    --instr-profile=json5format.profdata -format=html -output-dir=./covgen/$1/

rm formatjson5.profraw
rm json5format.profdata
