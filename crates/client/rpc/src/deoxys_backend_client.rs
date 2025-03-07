use mc_db::{DbError, DeoxysBackend};
use mp_block::DeoxysBlock;
use mp_digest_log::find_starknet_block;
use mp_types::block::{DBlockT, DHashT};
use sc_client_api::backend::{Backend, StorageProvider};
use sp_api::BlockId;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use starknet_api::hash::StarkHash;

use crate::errors::StarknetRpcApiError;

pub fn load_hash<C>(client: &C, hash: StarkHash) -> Result<Option<DHashT>, DbError>
where
    C: HeaderBackend<DBlockT> + 'static,
{
    let substrate_hashes = DeoxysBackend::mapping().substrate_block_hash(hash)?;

    if let Some(substrate_hashes) = substrate_hashes {
        for substrate_hash in substrate_hashes {
            if is_canon::<C>(client, substrate_hash) {
                return Ok(Some(substrate_hash));
            }
        }
    }

    Ok(None)
}

pub fn is_canon<C>(client: &C, target_hash: DHashT) -> bool
where
    C: HeaderBackend<DBlockT> + 'static,
{
    if let Ok(Some(number)) = client.number(target_hash) {
        if let Ok(Some(hash)) = client.hash(number) {
            return hash == target_hash;
        }
    }
    false
}

// Get a starknet block from a substrate hash.
// # Arguments
// * `client` - The Deoxys client
// * `overrides` - The OverrideHandle
// * `target_number` - A substrate block hash
//
// # Returns
// * `Result<Block, StarknetRpcApiError>` - A Result with the corresponding Starknet block
// or Error.
pub fn starknet_block_from_substrate_hash<B: BlockT, C, BE>(
    client: &C,
    target_number: <<B>::Header as HeaderT>::Number,
) -> Result<DeoxysBlock, StarknetRpcApiError>
where
    B: BlockT,
    BE: Backend<B> + 'static,
    C: HeaderBackend<B> + StorageProvider<B, BE> + 'static,
{
    let substrate_block_hash = client.block_hash_from_id(&BlockId::Number(target_number));

    match substrate_block_hash {
        Ok(Some(block_hash)) => {
            let block = get_block_by_block_hash(client, block_hash).unwrap();

            Ok(block)
        }
        _ => Err(StarknetRpcApiError::BlockNotFound),
    }
}

/// Returns the current Starknet block from the block header's digest
pub fn get_block_by_block_hash<B, C>(
    client: &C,
    substrate_block_hash: <B as BlockT>::Hash,
) -> anyhow::Result<DeoxysBlock>
where
    B: BlockT,
    C: HeaderBackend<B>,
{
    let header = client
        .header(substrate_block_hash)
        .ok()
        .flatten()
        .ok_or_else(|| anyhow::Error::msg("Failed to retrieve header"))?;
    let digest = header.digest();
    let block = find_starknet_block(digest)?;
    Ok(block)
}
