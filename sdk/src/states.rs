use crate::*;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub(crate) enum Error {
    Recoverable { height: u32, depth: u32 },
    DuplicateBlock { height: u32, hash: String },
    Unrecoverable,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Recoverable { height, depth } => {
                write!(f, "{depth} block deep reorg detected at height {height}")
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
    new_block: NewBlockInfo,
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
                ic_cdk::println!("Detected reorg - depth: {}", reorg_depth,);
                if reorg_depth > finalize_threshold {
                    ic_cdk::println!("Reorg depth is greater than the max recoverable reorg depth");
                    return Err(Error::Unrecoverable);
                }
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
                        hash: new_block.block_hash,
                    });
                }
                return Err(Error::Recoverable {
                    height: current_block.block_height,
                    depth: reorg_depth,
                });
            }
        }
    }
}

fn handle_reorg(
    blocks: &mut BlockStorage,
    unconfirmed: &mut UnconfirmedTxStorage,
    height: u32,
    depth: u32,
) -> Result<(), Error> {
    ic_cdk::println!("rolling back state after reorg of depth {depth} at height {height}");
    for h in (height - depth + 1..=height).rev() {
        ic_cdk::println!("rolling back change record at height {h}");
        if let Some(reverted) = blocks.remove(&h) {
            for tx in reverted.txs.into_iter() {
                ic_cdk::println!(
                    "rollback finalized txid: {} with pools: {:?}",
                    tx.txid,
                    tx.pools
                );
                // Revert transaction state in each affected pool
                unconfirmed.insert(tx.txid, tx);
            }
        }
    }
    ic_cdk::println!(
        "successfully rolled back state to height {}",
        height - depth,
    );
    Ok(())
}

pub fn new_block<P>(
    blocks: &mut BlockStorage,
    unconfirmed: &mut UnconfirmedTxStorage,
    pools: &mut PoolStorage<P::PoolState>,
    global_state: &mut BlockStateStorage<P::GlobalState>,
    args: NewBlockArgs,
) -> NewBlockResponse
where
    P: Hook,
{
    // Check for blockchain reorganizations
    match detect_reorg(blocks, P::finalize_threshold(), args.clone()) {
        Ok(_) => {}
        Err(Error::DuplicateBlock { height, hash }) => {
            ic_cdk::println!(
                "Duplicate block detected at height {} with hash {}",
                height,
                hash
            );
        }
        Err(Error::Unrecoverable) => {
            return Err("Unrecoverable reorg detected".to_string());
        }
        Err(Error::Recoverable { height, depth }) => {
            handle_reorg(blocks, unconfirmed, height, depth).map_err(|e| format!("{:?}", e))?;
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
    let mut recent = global_state.last_key_value().map(|(_, v)| v.inner);
    P::on_block_confirmed(&mut recent, block.clone());
    if let Some(r) = recent {
        global_state.insert(block_height, GlobalStateWrapper::new(r));
    }

    blocks.insert(block_height, block);

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
                    let mut pool = pools.get(&addr).ok_or(format!(
                        "Pool {} not found but marked an associated transaction {}",
                        addr, tx.txid
                    ))?;
                    pool.finalize(tx.txid)?;
                    // override the pool
                    pools.insert(addr.clone(), pool);
                    // P::on_tx_finalized(addr, *txid, finalized_block.clone());
                }
            }
        }
    }

    // Clean up old block data that's no longer needed
    let mut heights_to_remove: Vec<u32> = blocks
        .iter()
        .map(|entry| entry.into_pair())
        .take_while(|(height, _)| *height <= confirmed_height)
        .map(|(height, _)| height)
        .collect();
    heights_to_remove.sort();
    for height in heights_to_remove {
        ic_cdk::println!("removing block: {}", height);
        if let Some(_block) = blocks.remove(&height) {
            // P::on_block_finalized(&mut recent, block);
        }
    }
    Ok(())
}

pub fn rollback_tx<P>(
    unconfirmed: &mut UnconfirmedTxStorage,
    pools: &mut PoolStorage<P::PoolState>,
    args: RollbackTxArgs,
) -> RollbackTxResponse
where
    P: Hook,
{
    if let Some(record) = unconfirmed.remove(&args.txid) {
        ic_cdk::println!(
            "rollback unconfirmed txid: {} with pools: {:?}",
            args.txid,
            record.pools
        );
        // Roll back each affected pool to its state before this transaction
        for addr in record.pools.iter() {
            let mut pool = pools.get(addr).ok_or(format!(
                "Pool {} not found but marked an associated transaction {}",
                addr, args.txid
            ))?;
            // TODO?
            pool.rollback(args.txid)
                .map_err(|e| format!("Failed to rollback pool {}: {}", addr, e))?;
            pools.insert(addr.clone(), pool);
        }
    }
    Ok(())
}
