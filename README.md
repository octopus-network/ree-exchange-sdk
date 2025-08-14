# REE Exchange Rust SDK

[![docs.rs](https://img.shields.io/docsrs/ree-exchange-sdk)](https://docs.rs/ree-exchange-sdk/latest/ree_exchange_sdk/)

> The Rust SDK for building native Bitcoin dApps on REE(Runes Exchange Environment).

Unlike Ethereum and other smart contract platforms, Bitcoin's scripting language is not Turing complete, making it extremely challenging—if not impossible—to develop complex applications like AMM protocols directly on the Bitcoin network using BTC scripts and the UTXO model.

REE overcomes this limitation by leveraging the powerful Chain Key technology of the Internet Computer Protocol (ICP) and Bitcoin's Partially Signed Bitcoin Transactions (PSBT) to extend the programmability of Bitcoin's Rune assets.

## Basic procedures of REE exchange

**Constructing the PSBT**: The REE exchange client application (e.g., a wallet or interface) gathers the necessary information from the REE exchange and constructs a PSBT based on the user’s input. The user then signs the PSBT to authorize the transaction.

**Submitting the PSBT to REE**: The client composes the signed PSBT and essential information retrieved in the previous step and submit to REE Orchestrator. REE will validate the PSBT(including the UTXOs and their RUNE information) and analysis the input-output relations. If all check pass, Orchestrator will forward the request to RichSwap.

**Exchange's Validation and Signing**: The exchange verifies the transaction details from REE Orchestrator and, if everything is valid, signs the pool’s UTXO using the ICP Chain Key. This step transforms the PSBT into a fully valid Bitcoin transaction.

**Broadcasting the Transaction**: The finalized transaction is returned to the REE, which broadcasts it to the Bitcoin network for execution.


