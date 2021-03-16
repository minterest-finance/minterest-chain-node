.PHONY: init
init: toolchain submodule build

.PHONY: toolchain
toolchain:
	./scripts/init.sh

.PHONY: submodule
submodule:
	git submodule update --init --recursive

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
	cargo run -- --dev -lruntime=debug

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

.PHONY: cargo-dups
cargo-dups:
	cargo tree --duplicate

update-orml:
	cd orml && git checkout master && git pull
	git add orml

update: update-orml
	cargo update
	make check
