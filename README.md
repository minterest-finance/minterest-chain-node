# Minterest chain node

Released version: **0.5.0** <br>
Developed version: **0.6.0**

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

The project has current under development version set up in [cargo.toml](cargo.toml) file. This version should be upgraded once a tag for the previous version is released and the team starts a new version under development. As said - **the first commit** after the release is a commit to [cargo.toml](cargo.toml) with the next under development version number.

Each pallet and module has its own version which is not greater than the main one. Pallet version is upgraded once a significant change is made or a new feature is developed upon that pallet. **Pallet version should be upgraded in the same commit where the change was performed.**

## Example:

New version under development should be 0.6.0. The first commit in master branch after the previous version was cut is a cahnge of version to 0.6.0 in cargo.toml.

The pallets versions for pallet A and B are 0.5.11 and 0.5.12. 

During development the team changes pallet B and increases it's version to 0.6.0. Pallet A remains with 0.5.11.

Ongoing development causes pallet B version to grow to 0.6.21.

To form a release, last commit in master should be marked with 0.6.0.

Next commit to master is a cargo.toml file change with version 0.7.0, and the process repeats.

## Release process

To form a relase, follow the steps:
* Tag the last commit with a version tag from [cargo.toml](cargo.toml). This marks the end of previous version development.
* Push a new version in Cargo.toml and this readme to mark the beginning of development of a new version.

In case of hot fixes create a separate branch from tagged commit and work there. Merge the fixes branch back to master too.
