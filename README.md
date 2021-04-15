# Minterest chain node

Released version: **0.6.1** <br>
Developed version: **0.6.2**

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
./target/release/minterest -h
```

## Run

The `make run` command will launch the single-node development chain with persistent state. After the project has been built, there are other ways to launch the node.

### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/minterest --dev
```

This command will launch a temporary node and its state will be discarded after you terminate the process:

```bash
./target/release/minterest --dev --tmp
```

Start the development chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/minterest -lruntime=debug --dev
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

# Versioning

This repo supports versioning system organized in the next way:

1. Patch version (0.5.1, 0.5.2) represents patch level for bugfixes, currently developing features and intermediate tags.

2. Minor version (0.5.0, 0.6.0) represents features developed and released.

3. Major version (1.0.0, 2.0.0) represents release of a scope of features within a stable build which is ready for production.

The project has latest cut version in [Cargo.toml](cargo.toml) file in the project root. This means that current version in development is later then the one in Cargo.toml

## Release process

To form a release, follow the steps:
* Update Cargo.toml in the project root and all internal pallets. The versions should be the same across the files.
* Commit the changes as a separate MR. 
* Tag this commit with the same version.
