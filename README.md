# REE Types

This repository contains the essential data type definitions for REE (Runes Exchange Environment).

## Exchange Interfaces

In REE, every exchange must implement the following six functions:

| Function Name      | Parameters               | Return Type           | Description |
|-------------------|------------------------|----------------------|-------------|
| `get_pool_list`   | `GetPoolListArgs`       | `Vec<PoolOverview>`  | See [Get Pool List](#get-pool-list). |
| `get_pool_info`   | `GetPoolInfoArgs`       | `Option<PoolInfo>`   | See [Get Pool Info](#get-pool-info). |
| `get_minimal_tx_value` | `GetMinimalTxValueArgs` | `Result<u64, String>` | See [Get Minimal Tx Value](#get-minimal-tx-value). |
| `execute_tx`      | `ExecuteTxArgs`         | `Result<String, String>` | See [Execute Tx](#execute-tx). |
| `finalize_tx`     | `FinalizeTxArgs`        | `Result<(), String>`  | See [Finalize Tx](#finalize-tx). |
| `rollback_tx`     | `RollbackTxArgs`        | `Result<(), String>`  | See [Rollback Tx](#rollback-tx). |

Implementation Notes:

- The REE Orchestrator calls these functions to interact with exchanges **WITHOUT** attaching any cycles.
- Every exchange **MUST** implement these functions **exactly as defined** in this repository. Failure to do so will prevent the exchange from being registered in the REE Orchestrator, or may cause a registered exchange to be halted.
- These functions may be implemented as `async` or synchronous.
- They may be declared with `#[ic_cdk::query]` or `#[ic_cdk::update]` in the exchange canister.
- All parameters and return types are defined in the `ree_types::exchange_interfaces` module.

### Get Pool List

Returns the list of pools maintained by the exchange.

Parameters:

```rust
pub struct GetPoolListArgs {
    pub from: Option<Pubkey>,
    pub limit: u32,
}
```

Return Type:

```rust
pub struct PoolOverview {
    pub key: Pubkey,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub btc_reserved: u64,
    pub coin_reserved: Vec<CoinBalance>,
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

Return Type:

```rust
pub struct PoolInfo {
    pub key: Pubkey,
    pub name: String,
    pub address: String,
    pub nonce: u64,
    pub coin_reserved: Vec<CoinBalance>,
    pub btc_reserved: u64,
    pub utxos: Vec<Utxo>,
    pub attributes: String,
}
```

### Get Minimal Tx Value

Returns the minimum transaction value that can be accepted by the exchange, considering the zero-confirmation transaction queue length for a specific pool.

Parameters:

```rust
pub struct GetMinimalTxValueArgs {
    pub pool_address: String,
    pub zero_confirmed_tx_queue_length: u32,
}
```

Return Type:

- `Ok(u64)`: The minimal transaction value in `sats`.
- `Err(String)`: An error message if retrieval fails.

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

- `Ok(String)`: The signed PSBT data in hex format if required.
- `Err(String)`: An error message if execution fails.

### Finalize Tx

Finalizes a transaction in the exchange. **All transactions preceding the given transaction should also be considered finalized.**

Parameters:

```rust
pub struct FinalizeTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}
```

Return Type:

- `Ok(())`: On successful finalization.
- `Err(String)`: If an error occurs.

### Rollback Tx

Rolls back a transaction in the exchange. **All transactions following the given transaction should also be considered canceled.**

Parameters:

```rust
pub struct RollbackTxArgs {
    pub pool_key: Pubkey,
    pub txid: Txid,
}
```

Return Type:

- `Ok(())`: On successful rollback.
- `Err(String)`: If an error occurs.
