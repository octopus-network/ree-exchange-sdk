# REE Types

This repository contains the essential data type definitions for REE (Runes Exchange Environment).

## Exchange Interfaces

In REE, every exchange must implement the following six functions:

| Function Name      | Parameters               | Return Type           | Description |
|-------------------|------------------------|----------------------|-------------|
| `get_pool_list`   | -       | `Vec<PoolInfo>`  | See [Get Pool List](#get-pool-list). |
| `get_pool_info`   | `GetPoolInfoArgs`       | `Option<PoolInfo>`   | See [Get Pool Info](#get-pool-info). |
| `get_minimal_tx_value` | `GetMinimalTxValueArgs` | `u64` | See [Get Minimal Tx Value](#get-minimal-tx-value). |
| `execute_tx`      | `ExecuteTxArgs`         | `Result<String, String>` | See [Execute Tx](#execute-tx). |
| `unconfirm_tx`     | `UnconfirmTxArgs`        | `Result<(), String>`  | See [Unconfirm Tx](#unconfirm-tx). |
| `rollback_tx`     | `RollbackTxArgs`        | `Result<(), String>`  | See [Rollback Tx](#rollback-tx). |
| `new_block`     | `NewBlockArgs`        | `Result<(), String>`  | See [New Block](#new-block). |

Implementation Notes:

- The REE Orchestrator calls these functions to interact with exchanges **WITHOUT** attaching any cycles.
- Every exchange **MUST** implement these functions **exactly as defined** in this repository. Failure to do so will prevent the exchange from being registered in the REE Orchestrator, or may cause a registered exchange to be halted.
- These functions may be implemented as `async` or synchronous.
- The `get_pool_list`, `get_pool_info` and `get_minimal_tx_value` may be declared with `#[ic_cdk::query]` or `#[ic_cdk::update]` in the exchange canister. The other functions **MUST** be declared with `#[ic_cdk::update]`.
- All parameters and return types are defined in the `ree_types::exchange_interfaces` module.

### Get Pool List

Returns the list of pools maintained by the exchange.

Return Type: `Vec<PoolInfo>`, where `PoolInfo` is defined as:

```rust
pub struct PoolInfo {
    pub key: Pubkey,
    pub key_derivation_path: Vec<Vec<u8>>,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}
```

### Get Pool Info

Returns detailed information about a specified pool.

Parameters:

```rust
pub struct GetPoolInfoArgs {
    pub pool_address: String,
}
```

Return Type: `Option<PoolInfo>`

### Get Minimal Tx Value

Returns the minimum transaction value that can be accepted by the exchange, considering the zero-confirmation transaction queue length for a specific pool.

Parameters:

```rust
pub struct GetMinimalTxValueArgs {
    pub pool_address: String,
    pub zero_confirmed_tx_queue_length: u32,
}
```

Return Type: `u64`, the minimal transaction value in `sats`.

### Execute Tx

Executes a transaction in the exchange.

Parameters:

```rust
pub struct ExecuteTxArgs {
    pub psbt_hex: String,
    pub txid: Txid,
    pub intention_set: IntentionSet,
    pub intention_index: u32,
    pub zero_confirmed_tx_queue_length: u32,
}
```

Return Type:

- `Ok(String)`: The signed PSBT data in hex format. The exchange can add corresponding signature(s) to the PSBT data or not, but a valid PSBT data with the same `txid` with the given `psbt_hex` **MUST** be returned.
- `Err(String)`: An error message if execution fails.

### Unconfirm Tx

Unconfirm a previously confirmed transaction in the exchange.

Parameters:

```rust
pub struct UnconfirmTxArgs {
    pub txid: Txid,
}
```

Return Type:

- `Ok(())`: On success.
- `Err(String)`: If an error occurs.

### Rollback Tx

Rolls back a transaction in the exchange. **All transactions following the given transaction should also be considered canceled.**

Parameters:

```rust
pub struct RollbackTxArgs {
    pub txid: Txid,
}
```

Return Type:

- `Ok(())`: On success.
- `Err(String)`: If an error occurs.

### New Block

Notifies the exchange of a new block. The `confirmed_txids` are an array of txid which are executed by the exchange previously, these txids are included in the given block. The exchange can use this information to update its internal state.

Parameters:

```rust
pub struct NewBlockArgs {
    pub block_height: u64,
    pub block_hash: String,
    pub block_timestamp: u64,
    pub confirmed_txids: Vec<Txid>,
}
```

Return Type:

- `Ok(())`: On success.
- `Err(String)`: If an error occurs.
