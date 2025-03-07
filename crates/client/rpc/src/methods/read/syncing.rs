use jsonrpsee::core::RpcResult;
use mc_db::DeoxysBackend;
use mp_hashers::HasherT;
use mp_types::block::DBlockT;
use sc_client_api::backend::{Backend, StorageProvider};
use sc_client_api::BlockBackend;
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_blockchain::HeaderBackend;
use starknet_core::types::{SyncStatus, SyncStatusType};

use crate::errors::StarknetRpcApiError;
use crate::{deoxys_backend_client, Starknet};

/// Returns an object about the sync status, or false if the node is not synching
///
/// ### Arguments
///
/// This function does not take any arguments.
///
/// ### Returns
///
/// * `Syncing` - An Enum that can either be a `mc_rpc_core::SyncStatus` struct representing the
///   sync status, or a `Boolean` (`false`) indicating that the node is not currently synchronizing.
///
/// This is an asynchronous function due to its reliance on `sync_service.best_seen_block()`,
/// which potentially involves network communication and processing to determine the best block
/// seen by the sync service.
pub async fn syncing<BE, C, H>(starknet: &Starknet<BE, C, H>) -> RpcResult<SyncStatusType>
where
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    H: HasherT + Send + Sync + 'static,
{
    // obtain best seen (highest) block number
    match starknet.sync_service.best_seen_block().await {
        Ok(best_seen_block) => {
            let best_number = starknet.client.info().best_number;
            let highest_number = best_seen_block.unwrap_or(best_number);

            // get a starknet block from the starting substrate block number
            let starting_block = deoxys_backend_client::starknet_block_from_substrate_hash(
                starknet.client.as_ref(),
                starknet.starting_block,
            );

            // get a starknet block from the current substrate block number
            let current_block =
                deoxys_backend_client::starknet_block_from_substrate_hash(starknet.client.as_ref(), best_number);

            // get a starknet block from the highest substrate block number
            let highest_block =
                deoxys_backend_client::starknet_block_from_substrate_hash(starknet.client.as_ref(), highest_number);

            if starting_block.is_ok() && current_block.is_ok() && highest_block.is_ok() {
                // Convert block numbers and hashes to the respective type required by the `syncing` endpoint.
                let starting_block_num = UniqueSaturatedInto::<u64>::unique_saturated_into(starknet.starting_block);
                let starting_block_hash = starting_block?.header().hash::<H>().0;

                let current_block_num = UniqueSaturatedInto::<u64>::unique_saturated_into(best_number);
                let current_block_hash = current_block?.header().hash::<H>().0;

                // Get the highest block number and hash
                let (highest_block_hash, highest_block_num) =
                    DeoxysBackend::meta().get_latest_block_hash_and_number().map_err(|e| {
                        log::error!("Failed to retrieve the highest block hash and number: {}", e);
                        StarknetRpcApiError::InternalServerError
                    })?;

                // Build the `SyncStatus` struct with the respective syn information
                Ok(SyncStatusType::Syncing(SyncStatus {
                    starting_block_num,
                    starting_block_hash,
                    current_block_num,
                    current_block_hash,
                    highest_block_num,
                    highest_block_hash,
                }))
            } else {
                // If there was an error when getting a starknet block, then we return `false`,
                // as per the endpoint specification
                log::error!("Failed to load Starknet block");
                Ok(SyncStatusType::NotSyncing)
            }
        }
        Err(_) => {
            // If there was an error when getting a starknet block, then we return `false`,
            // as per the endpoint specification
            log::error!("`SyncingEngine` shut down");
            Ok(SyncStatusType::NotSyncing)
        }
    }
}
