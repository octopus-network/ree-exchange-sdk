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

## Quick start

If you are familier with IC canister development, you could easily create an empty rust crate and paste the code into the `lib.rs`.

``` rust
use candid::{CandidType, Deserialize};
use ic_cdk::{query, update};
use ic_stable_structures::{Storable, storable::Bound};
use ree_exchange_sdk::{
    prelude::*,
    {CoinBalance, Txid, Utxo},
};
use serde::Serialize;

#[derive(CandidType, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct DummyPoolState {
    pub txid: Txid,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}

impl Storable for DummyPoolState {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(self, &mut bytes);
        std::borrow::Cow::Owned(bytes)
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut bytes = vec![];
        let _ = ciborium::ser::into_writer(&self, &mut bytes);
        bytes
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let dire = ciborium::de::from_reader(bytes.as_ref()).expect("failed to decode Pool");
        dire
    }
}

impl StateView for DummyPoolState {
    fn inspect_state(&self) -> StateInfo {
        StateInfo {
            txid: self.txid,
            nonce: self.nonce,
            coin_reserved: self.coin_reserved.clone(),
            btc_reserved: self.btc_reserved,
            utxos: self.utxos.clone(),
            attributes: "{}".to_string(),
        }
    }
}

#[exchange]
pub mod exchange {
    use super::*;

    #[pools]
    pub struct DummyPools;

    impl Pools for DummyPools {
        type State = DummyPoolState;

        const POOL_MEMORY: u8 = 102;

        const BLOCK_MEMORY: u8 = 100;

        const TRANSACTION_MEMORY: u8 = 101;

        fn network() -> Network {
            Network::Testnet4
        }

        // This is optional
        fn finalize_threshold() -> u32 {
            60
        }
    }

    /// This is optional
    #[hook]
    impl Hook for DummyPools {
        /// This function is called when a new block is received, before any processing.
        fn on_block_received(_args: NewBlockInfo) {}

        /// This function is called when a transaction is dropped from the mempool.
        fn on_state_reverted(_address: String, _txid: Txid) {}

        /// This function is called when a transaction is confirmed in a block.
        fn on_state_confirmed(_address: String, _txid: Txid, _block: Block) {}

        /// This function is called when a transaction reaches the finalize threshold.
        fn on_state_finalized(_address: String, _txid: Txid, _block: Block) {}

        /// This function is called after a new block is processed.
        fn on_block_processed(_args: NewBlockInfo) {}
    }

    #[update]
    pub fn new_pool(args: Metadata) {
        let pool = Pool::new(args.clone());
        DummyPools::insert(pool.clone());
    }

    #[query]
    pub fn pre_swap(addr: String) -> Option<StateInfo> {
        DummyPools::get(&addr).and_then(|pool| {
            pool.states()
                .iter()
                .map(|s| s.inspect_state())
                .last()
                .clone()
        })
    }

    #[action(name = "swap")]
    pub fn execute_swap(_args: ExecuteTxArgs) -> ExecuteTxResponse {
        Ok("Transaction executed successfully".to_string())
    }
}
```

## REE exchange client

To complete an REE transaction, the exchange client should call REE Orchestrator `invoke` rather than making request into exchanges directly.

The `invoke` function of Orchestrator takes `InvokeArgs` as a parameter, which includes the following fields:

```rust
pub struct InvokeArgs {
    pub psbt_hex: String,
    pub intention_set: IntentionSet,
    pub initiator_utxo_proof: Vec<u8>,
}
```

Where `IntentionSet` is defined as:

```rust
pub struct IntentionSet {
    pub initiator_address: String,
    pub tx_fee_in_sats: u64,
    pub intentions: Vec<Intention>,
}

pub struct Intention {
    pub exchange_id: String,
    pub action: String,
    pub action_params: String,
    pub pool_address: String,
    pub nonce: u64,
    pub pool_utxo_spent: Vec<String>,
    pub pool_utxo_received: Vec<String>,
    pub input_coins: Vec<InputCoin>,
    pub output_coins: Vec<OutputCoin>,
}

pub struct InputCoin {
    // The address of the owner of the coins
    pub from: String,
    pub coin: CoinBalance,
}

pub struct OutputCoin {
    // The address of the receiver of the coins
    pub to: String,
    pub coin: CoinBalance,
}
```

The `invoke` function returns a `Result<String, String>`, where:

- The `Ok` value is the `txid` of the final Bitcoin transaction, which will be formed and broadcasted.
- The `Err` value is an error message if the execution of `invoke` fails.

The `invoke` function will call the `execute_tx` function of the exchange canister(s) based on the provided `IntentionSet`. If all intentions are successfully executed, the function broadcasts the final Bitcoin transaction and returns the `txid`.

Before invoking the exchange canisters, the Orchestrator performs necessary validations on the `IntentionSet` to ensure it aligns with the provided PSBT data.

### Intention Details

Each `IntentionSet` can contain multiple `Intention` objects, reflecting the user's intentions. The `Intention` struct consists of the following fields:

- `exchange_id`: The identifier of a registered exchange responsible for executing the intention. The Orchestrator will validate this field.
- `action`: The specific action to be executed by the exchange. The Orchestrator will **NOT** validate this field.
- `action_params`: Parameters for the action, specific to the exchange. The Orchestrator will **NOT** validate this field.
- `pool_address`: The address of the exchange pool where the intention will be executed. The Orchestrator will validate this field.
- `nonce`: A nonce representing the pool state in the exchange. The Orchestrator will **NOT** validate this field.
- `pool_utxo_spent`: The UTXO(s) owned by the pool that will be spent in the intention. **The clients of REE can leave this field empty as the Orchestrator will fill it in with the UTXO(s) that the pool will spend in the final Bitcoin transaction.**
- `pool_utxo_received`: The UTXO(s) that the pool will receive as part of the intention. These UTXOs should correspond to the outputs of the final Bitcoin transaction. **The clients of REE can leave this field empty as the Orchestrator will fill it in with the UTXO(s) that the pool will receive in the final Bitcoin transaction.**
- `input_coins`: The coins that will be spent in the intention. These should appear as inputs in the final Bitcoin transaction.
- `output_coins`: The coins that will be received in the intention. These should appear as outputs in the final Bitcoin transaction.
