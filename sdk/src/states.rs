use crate::*;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub(crate) enum Error {
    Recoverable { from: u32, to: u32 },
    DuplicateBlock { height: u32, hash: String },
    Unrecoverable,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Recoverable { from, to } => {
                write!(f, "reorg blocks from {from} to {to}")
            }
            Self::DuplicateBlock { height, hash } => {
                write!(
                    f,
                    "duplicate block detected at height {height} with hash {hash}"
                )
            }
            Self::Unrecoverable => write!(f, "unrecoverable reorg detected"),
        }
    }
}

impl std::error::Error for Error {}

fn detect_reorg(
    blocks: &BlockStorage,
    finalize_threshold: u32,
    new_block: &NewBlockInfo,
) -> Result<(), Error> {
    ic_cdk::println!(
        "Processing new block - height: {}, hash: {}, timestamp: {}, confirmed_txs: {}",
        new_block.block_height,
        new_block.block_hash,
        new_block.block_timestamp,
        new_block.confirmed_txids.len()
    );
    let current_block = blocks.last_key_value().map(|(_, v)| v);
    match current_block {
        None => {
            ic_cdk::println!("No blocks found in exchange - this is expected for new exchanges");
            return Ok(());
        }
        Some(current_block) => {
            ic_cdk::println!(
                "Current block: height: {:?}, hash: {:?}, timestamp: {:?}",
                current_block.block_height,
                current_block.block_hash,
                current_block.block_timestamp
            );
            if new_block.block_height == current_block.block_height + 1 {
                ic_cdk::println!("New block is the next block in the chain");
                return Ok(());
            } else if new_block.block_height > current_block.block_height + 1 {
                ic_cdk::println!("New block is more than one block ahead of the current block");
                return Err(Error::Unrecoverable);
            } else {
                let reorg_depth = current_block.block_height - new_block.block_height + 1;
                let target_block = blocks
                    .get(&new_block.block_height)
                    .ok_or(Error::Unrecoverable)
                    .inspect_err(|_| {
                        ic_cdk::println!(
                            "Detected reorg at {}, but it was removed.",
                            new_block.block_height
                        )
                    })?;
                if target_block.block_hash == new_block.block_hash {
                    ic_cdk::println!("New block is a duplicate block");
                    return Err(Error::DuplicateBlock {
                        height: new_block.block_height,
                        hash: new_block.block_hash.clone(),
                    });
                }
                ic_cdk::println!(
                    "Reorg detected from {} to {}",
                    new_block.block_height,
                    current_block.block_height
                );
                if reorg_depth > finalize_threshold {
                    ic_cdk::println!("Reorg depth is greater than the max recoverable reorg depth");
                    return Err(Error::Unrecoverable);
                }
                return Err(Error::Recoverable {
                    from: new_block.block_height,
                    to: current_block.block_height,
                });
            }
        }
    }
}

fn handle_reorg<P>(
    block_states: &mut BlockStateStorage<P::BlockState>,
    blocks: &mut BlockStorage,
    unconfirmed: &mut UnconfirmedTxStorage,
    from: u32,
    to: u32,
) -> Result<(), Error>
where
    P: Pools,
{
    // remove block state
    (from..=to).for_each(|h| {
        block_states.remove(&h);
    });
    // Rollback confirmed transactions
    (from..=to).rev().for_each(|h| {
        if let Some(reverted) = blocks.remove(&h) {
            for tx in reverted.txs.into_iter() {
                ic_cdk::println!(
                    "Rollback confirmed txid: {} with pools: {:?}",
                    tx.txid,
                    tx.pools
                );
                // The transaction is now unconfirmed again
                unconfirmed.insert(tx.txid, tx);
            }
        }
    });
    ic_cdk::println!("successfully rolled back state to {}", to,);
    Ok(())
}

pub fn confirm_txs<P>(
    block_states: &mut BlockStateStorage<P::BlockState>,
    blocks: &mut BlockStorage,
    unconfirmed: &mut UnconfirmedTxStorage,
    args: NewBlockArgs,
) -> Result<Option<Block>, String>
where
    P: Hook,
{
    P::pre_block_confirmed(args.block_height);
    // Check for blockchain reorganizations
    match detect_reorg(blocks, P::finalize_threshold(), &args) {
        Ok(_) => {}
        Err(Error::DuplicateBlock { height, hash }) => {
            ic_cdk::println!("Ignored duplicated block {}({}).", height, hash);
            return Ok(None);
        }
        Err(Error::Unrecoverable) => {
            return Err("Unrecoverable reorg detected".to_string());
        }
        Err(Error::Recoverable { from, to }) => {
            handle_reorg::<P>(block_states, blocks, unconfirmed, from, to)
                .map_err(|e| format!("{:?}", e))?;
        }
    }
    let NewBlockArgs {
        block_height,
        block_hash,
        block_timestamp,
        confirmed_txids,
    } = args;
    // Mark transactions as confirmed
    let mut confirmed = vec![];
    for txid in confirmed_txids.into_iter() {
        if let Some(record) = unconfirmed.remove(&txid) {
            ic_cdk::println!("confirm txid: {} with pools: {:?}", txid, record.pools);
            confirmed.push(record);
        }
    }
    let block = Block {
        block_height,
        block_hash,
        block_timestamp,
        txs: confirmed,
    };
    for tx in block.txs.iter() {
        for addr in tx.pools.iter() {
            P::on_tx_confirmed(addr.to_string(), tx.txid, block.clone());
        }
    }
    Ok(Some(block))
}

pub fn accept_block<P>(
    block_states: &mut BlockStateStorage<P::BlockState>,
    blocks: &mut BlockStorage,
    pools: &mut PoolStorage<P::PoolState>,
    block: Block,
) -> NewBlockResponse
where
    P: Pools,
{
    let block_height = block.block_height;
    blocks.insert(block.block_height, block);

    // Calculate the height below which blocks are considered fully confirmed (beyond reorg risk)
    let confirmed_height = block_height - P::finalize_threshold() + 1;

    // Finalize transactions in confirmed blocks
    for entry in blocks.iter() {
        let (height, block_info) = entry.into_pair();
        if height <= confirmed_height {
            ic_cdk::println!("finalizing txs in block: {}", height);
            for tx in block_info.txs.iter() {
                ic_cdk::println!("finalize txid: {} with pools: {:?}", tx.txid, tx.pools);
                // Make transaction state permanent in each affected pool
                for addr in tx.pools.iter() {
                    if let Some(mut pool) = pools.get(&addr) {
                        pool.finalize(tx.txid)?;
                        // override the pool
                        pools.insert(addr.clone(), pool);
                    }
                }
            }
        }
    }
    // Clean up old block data that's no longer needed
    let removing = blocks
        .keys()
        .take_while(|h| *h <= confirmed_height)
        .collect::<Vec<_>>();
    for height in removing.iter() {
        blocks.remove(&height);
    }
    for height in removing.iter() {
        if block_states.len() > 1 {
            block_states.remove(&height);
        }
    }
    Ok(())
}

pub fn reject_tx<P>(
    unconfirmed: &mut UnconfirmedTxStorage,
    pools: &mut PoolStorage<P::PoolState>,
    args: RollbackTxArgs,
) -> RollbackTxResponse
where
    P: Hook,
{
    if let Some(tx) = unconfirmed.remove(&args.txid) {
        ic_cdk::println!(
            "rollback unconfirmed tx {} with pools: {:?}",
            tx.txid,
            tx.pools
        );
        return rollback_tx::<P>(pools, tx, args.reason_code);
    }
    Ok(())
}

fn rollback_tx<P>(
    pools: &mut PoolStorage<P::PoolState>,
    tx: TxRecord,
    reason: String,
) -> RollbackTxResponse
where
    P: Hook,
{
    // Roll back each affected pool to its state before this transaction
    for addr in tx.pools.iter() {
        let mut pool = pools.get(addr).ok_or(format!(
            "Pool {} not found but marked an associated transaction {}",
            addr, tx.txid
        ))?;
        let reverted = pool
            .rollback(tx.txid)
            .map_err(|e| format!("Failed to rollback pool {}: {}", addr, e))?;
        pools.insert(addr.clone(), pool);
        P::on_tx_rollbacked(addr.to_string(), tx.txid, reason.clone(), reverted);
    }
    Ok(())
}
