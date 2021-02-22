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
### Release process

To mark a relase, follow the steps:
* Master contains only those changes, which passed QA.
* Master branch code coverage has not decreased.
* Make sure the CI is green.
* Update pallets versions using semver in a separate MR. Merge this MR.
* Tag a commit with pallet updates with a version tag using semver.
* In case of hot fixes create a separate branch from tagged commit and work there. Merge the fixes branch back to master too.

## Semantic versioning

This repo supports versioning system organized in the next way:

1. Patch version (0.5.1, 0.5.2) represents patch level for bugfixes, currently developing features and intermediate tags.

2. Minor version (0.5.0, 0.6.0) represents features developed and released.

3. Major version (1.0.0, 2.0.0) represents release of a scope of features within a stable build which is ready for production.

### Versioninng

The project has current release version set up in [cargo.toml](cargo.toml) file. This version should be upgraded once a tag is released and the team starts a new version under development. As said - **the first commit** after the release is a commit to [cargo.toml](cargo.toml) and release with version update to set the release.

Each pallet and module has its own version which is not greater than the main one. Pallet version is upgraded once a significant change is made or a new feature is developed upon that pallet. Pallet version should be upgraded in the same commit where the change was performed.

Example:

Version under development is 0.6.0. The main version in cargo.toml is 0.5.12, pallets versions for pallet A and B are 0.5.11 and 0.5.12. When forming a release, a version tag should be placed on a commit with 0.6.0 version bump in cargo.toml. 

After this main cargo.toml version is 0.6.0 and pallets A and B still have 0.5.11 and 0.5.12. If any changes made to pallet B, it's version is set to 0.6.1.
