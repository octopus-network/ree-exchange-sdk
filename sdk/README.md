# REE Exchange Rust SDK

[![docs.rs](https://img.shields.io/docsrs/ree-exchange-sdk)](https://docs.rs/ree-exchange-sdk/latest/ree_exchange_sdk/)
[![Crates.io](https://img.shields.io/crates/v/ree-exchange-sdk.svg)](https://crates.io/crates/ree-exchange-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> The Rust SDK for building Bitcoin dApps on REE (Runes Exchange Environment).

## Overview

Bitcoin Script is intentionally not Turing-complete, which makes building complex protocols (e.g., AMMs) directly on the UTXO model very challenging. REE extends Bitcoin programmability for Rune assets by combining:

- ICP (Internet Computer Protocol) Chain Key cryptography
- PSBT (Partially Signed Bitcoin Transactions)

This repository is a Rust workspace that provides the SDK, proc-macros, and shared types required to build REE exchanges.

## Install

Add the dependency in your `Cargo.toml`:

```toml
[dependencies]
ree-exchange-sdk = "0.13"
```

## REE exchange flow (high level)

1. Construct PSBT: The client (wallet/UI) composes a PSBT from user input and REE exchange metadata, then the user signs it.
2. Submit to REE: The orchestrator validates the PSBT (including UTXO/Rune info) and checks I/O relations. On success, it forwards the request to the Exchange service.
3. Validate & Sign: The Exchange signs the pool UTXO using ICP Chain Key, turning the PSBT into a valid Bitcoin transaction.
4. Broadcast: The finalized transaction is returned to REE and broadcast to the Bitcoin network.

## License & links

- License: MIT (see `LICENSE`)
- Tutorials: https://docs.omnity.network/
- Website: https://www.omnity.network/
