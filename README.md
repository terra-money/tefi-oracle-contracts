# TeFi Oracle

This repository contains the source code for a set of oracle smart contracts running on [Terra](https://terra.money) blockchain.

## Contracts

| Contract                                                | Reference | Description                                                                                           |
| ------------------------------------------------------- | --------- | ----------------------------------------------------------------------------------------------------- |
| [`hub`](./contracts/oracle-hub)                         | [doc]()   | Central price source for all assets, redirects requests to proxies based on priority and availability |
| [`proxy-template`](./contracts/oracle-proxy-template)   | [doc]()   | Template of a contract that implements the proxy standard                                             |
| [`proxy-band`](./contracts/oracle-proxy-band)           | [doc]()   | Proxy contract for Band Protocol price sources                                                        |
| [`proxy-chainlink`](./contracts/oracle-proxy-chainlink) | [doc]()   | Proxy contract for Chainlink price sources                                                            |
| [`proxy-feed`](./contracts/oracle-proxy-feed)           | [doc]()   | Custom proxy contract that allows external sources to feed prices                                     |

## Development

### Environment Setup

- Rust v1.44.1+
- `wasm32-unknown-unknown` target
- Docker

1. Install `rustup` via https://rustup.rs/

2. Run the following:

```sh
rustup default stable
rustup target add wasm32-unknown-unknown
```

3. Make sure [Docker](https://www.docker.com/) is installed

### Unit / Integration Tests

Each contract contains Rust unit and integration tests embedded within the contract source directories. You can run:

```sh
cargo unit-test
cargo integration-test
```

### Compiling

After making sure tests pass, you can compile each contract with the following:

```sh
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

#### Production

For production builds, run the following:

```sh
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.11.5
```

or

```sh
sh build_release.sh
```

This performs several optimizations which can significantly reduce the final size of the contract binaries, which will be available inside the `artifacts/` directory.

## Formatting

Make sure you run `rustfmt` before creating a PR to the repo. You need to install the `nightly` version of `rustfmt`.

```sh
rustup toolchain install nightly
```

To run `rustfmt`,

```sh
cargo fmt
```

## Linting

You should run `clippy` also. This is a lint tool for rust. It suggests more efficient/readable code.
You can see [the clippy document](https://rust-lang.github.io/rust-clippy/master/index.html) for more information.
You need to install `nightly` version of `clippy`.

### Install

```sh
rustup toolchain install nightly
```

### Run

```sh
cargo clippy --all --all-targets -- -D warnings
```

## Testing

Developers are strongly encouraged to write unit tests for new code, and to submit new unit tests for old code. Unit tests can be compiled and run with: `cargo test --all`. For more details, please reference [Unit Tests](https://github.com/CodeChain-io/codechain/wiki/Unit-Tests).
