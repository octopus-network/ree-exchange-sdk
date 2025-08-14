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
    let current_block = blocks.iter().rev().next().map(|e| e.into_pair().1);
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
                let target_block = blocks.get(&new_block.block_height).unwrap();
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
    transactions: &mut TransactionStorage,
    height: u32,
    depth: u32,
) {
    ic_cdk::println!("rolling back state after reorg of depth {depth} at height {height}");

    for h in (height - depth + 1..=height).rev() {
        ic_cdk::println!("rolling back change record at height {h}");
        let block = blocks.get(&h).unwrap();
        for txid in block.confirmed_txids.iter() {
            if let Some(record) = transactions.remove(&(*txid, true)) {
                transactions.insert((*txid, false), record);
                ic_cdk::println!("unconfirm txid: {}", txid);
            }
        }
        blocks.remove(&h);
    }

    ic_cdk::println!(
        "successfully rolled back state to height {}",
        height - depth,
    );
}

pub fn rollback_tx<P>(
    transactions: &mut TransactionStorage,
    pools: &mut PoolStorage<P::State>,
    args: RollbackTxArgs,
) -> RollbackTxResponse
where
    P: Pools + Hook,
{
    // Look up the transaction record (both confirmed and unconfirmed)
    let maybe_unconfirmed_record = transactions.get(&(args.txid.clone(), false));
    let maybe_confirmed_record = transactions.get(&(args.txid.clone(), true));
    let record = maybe_confirmed_record
        .or(maybe_unconfirmed_record)
        .ok_or(format!("Txid not found: {}", args.txid))?;
    ic_cdk::println!(
        "rollback txid: {} with pools: {:?}",
        args.txid,
        record.pools
    );
    // Roll back each affected pool to its state before this transaction
    for addr in record.pools.iter() {
        pools
            .get(addr)
            .ok_or(format!(
                "Pool {} not found but marked an associated transaction {}",
                addr, args.txid
            ))?
            .rollback(args.txid)
            .map_err(|e| format!("Failed to rollback pool {}: {}", addr, e))?;
        P::on_tx_rollbacked(addr.clone(), args.txid, args.reason_code.clone());
    }
    transactions.remove(&(args.txid, false));
    transactions.remove(&(args.txid, true));
    Ok(())
}

pub fn new_block<P>(
    blocks: &mut BlockStorage,
    transactions: &mut TransactionStorage,
    pools: &mut PoolStorage<P::State>,
    args: NewBlockArgs,
) -> NewBlockResponse
where
    P: Pools + Hook,
{
    P::pre_new_block(args.clone());
    // Check for blockchain reorganizations
    match detect_reorg(blocks, P::finalize_threshold(), args.clone()) {
        Ok(_) => {}
        Err(crate::reorg::Error::DuplicateBlock { height, hash }) => {
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
            handle_reorg(blocks, transactions, height, depth);
        }
    }
    let NewBlockArgs {
        block_height,
        block_hash,
        block_timestamp,
        confirmed_txids,
    } = args.clone();

    let new_block = Block {
        height: block_height,
        hash: block_hash,
        timestamp: block_timestamp,
    };

    blocks.insert(block_height, args.clone());

    // Mark transactions as confirmed
    for txid in confirmed_txids {
        if let Some(record) = transactions.remove(&(txid, false)) {
            transactions.insert((txid, true), record.clone());
            ic_cdk::println!("confirm txid: {} with pools: {:?}", txid, record.pools);
            for addr in record.pools.into_iter() {
                P::on_tx_confirmed(addr, txid, new_block.clone());
            }
        }
    }
    // Calculate the height below which blocks are considered fully confirmed (beyond reorg risk)
    let confirmed_height = block_height - P::finalize_threshold() + 1;

    // Finalize transactions in confirmed blocks
    for entry in blocks.iter() {
        let (height, block_info) = entry.into_pair();
        let finalized_block = Block {
            height: block_info.block_height,
            hash: block_info.block_hash.clone(),
            timestamp: block_info.block_timestamp,
        };
        if height <= confirmed_height {
            ic_cdk::println!("finalizing txs in block: {}", height);
            for txid in block_info.confirmed_txids.iter() {
                if let Some(record) = transactions.get(&(*txid, true)) {
                    ic_cdk::println!("finalize txid: {} with pools: {:?}", txid, record.pools);
                    // Make transaction state permanent in each affected pool
                    for addr in record.pools.into_iter() {
                        pools
                            .get(&addr)
                            .ok_or(format!(
                                "Pool {} not found but marked an associated transaction {}",
                                addr, txid
                            ))?
                            .finalize(*txid)?;
                        P::on_tx_finalized(addr, *txid, finalized_block.clone());
                    }
                    transactions.remove(&(*txid, true));
                }
            }
        }
    }

    // Clean up old block data that's no longer needed
    let heights_to_remove: Vec<u32> = blocks
        .iter()
        .map(|entry| entry.into_pair())
        .take_while(|(height, _)| *height <= confirmed_height)
        .map(|(height, _)| height)
        .collect();
    for height in heights_to_remove {
        ic_cdk::println!("removing block: {}", height);
        blocks.remove(&height);
    }
    P::post_new_block(args);
    Ok(())
}
