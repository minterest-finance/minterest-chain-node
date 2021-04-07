.PHONY: init
init: toolchain build

.PHONY: toolchain
toolchain:
	./scripts/init.sh

.PHONY: build
build:
	cargo build --release

.PHONY: check
check:
	SKIP_WASM_BUILD=1 cargo check

.PHONY: test
test:
	SKIP_WASM_BUILD=1 cargo test --all

.PHONY: run
run:
	cargo build --release
	./target/release/minterest --dev --tmp

.PHONY: check-tests
check-tests:
	SKIP_WASM_BUILD=1 cargo check --tests --all

.PHONY: check-debug
check-debug:
	RUSTFLAGS="-Z macro-backtrace" SKIP_WASM_BUILD=1 cargo +nightly check

.PHONY: purge
purge: target/debug/node-minterest
	target/debug/node-minterest purge-chain --dev -y

.PHONY: restart
restart: purge run

.PHONY: cargo-audit
cargo-audit:
	cargo audit

.PHONY: update
update:
	cargo update
	make check
