# Minterest chain node


# Building & Running MinterestChain


## Prerequisites

Ensure you have `llvm` and `clang` installed. On Ubuntu:

```bash
apt install -y llvm clang
```

## Building

### Rust Setup

Setup instructions for working with the [Rust](https://www.rust-lang.org/) programming language can
be found at the
[Substrate Developer Hub](https://substrate.dev/docs/en/knowledgebase/getting-started). Follow those
steps to install [`rustup`](https://rustup.rs/) and configure the Rust toolchain to default to the
latest stable version.

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Make sure you have `submodule.recurse` set to true to make life with submodule easier.

```bash
git config --global submodule.recurse true
```

### Makefile

This project uses a [Makefile](Makefile) to document helpful commands and make it easier to execute
them. Get started by running these [`make`](https://www.gnu.org/software/make/manual/make.html)
targets:


Install required tools:

```bash
make init
```

Build all native code:

```bash
make build
```

### Embedded Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/node-template -h
```

## Run

The `make run` command will launch the single-node development chain with persistent state. After the project has been built, there are other ways to launch the node.

### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/node-template --dev
```

This command will launch a temporary node and its state will be discarded after you terminate the process:

```bash
./target/release/node-template --dev --tmp
```

Start the development chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/node-template -lruntime=debug --dev
```

## Development

To type check:

```bash
make check
```

To purge old chain data:

```bash
make purge
```

To purge old chain data and run

```bash
make restart
```

Update ORML

```bash
make update
```
