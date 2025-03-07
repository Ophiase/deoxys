use jsonrpsee::core::RpcResult;
use mp_hashers::HasherT;
use mp_types::block::DBlockT;
use sc_client_api::backend::{Backend, StorageProvider};
use sc_client_api::BlockBackend;
use sp_blockchain::HeaderBackend;
use starknet_core::types::BlockId;

use crate::deoxys_backend_client::get_block_by_block_hash;
use crate::Starknet;

/// Get the Number of Transactions in a Given Block
///
/// ### Arguments
///
/// * `block_id` - The identifier of the requested block. This can be the hash of the block, the
///   block's number (height), or a specific block tag.
///
/// ### Returns
///
/// * `transaction_count` - The number of transactions in the specified block.
///
/// ### Errors
///
/// This function may return a `BLOCK_NOT_FOUND` error if the specified block does not exist in
/// the blockchain.
pub fn get_block_transaction_count<BE, C, H>(starknet: &Starknet<BE, C, H>, block_id: BlockId) -> RpcResult<u128>
where
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    H: HasherT + Send + Sync + 'static,
{
    let substrate_block_hash = starknet.substrate_block_hash_from_starknet_block(block_id)?;

    let starknet_block = get_block_by_block_hash(starknet.client.as_ref(), substrate_block_hash)?;

    Ok(starknet_block.header().transaction_count)
}
