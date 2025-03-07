use jsonrpsee::core::RpcResult;
use mc_db::DeoxysBackend;
use mp_felt::Felt252Wrapper;
use mp_hashers::HasherT;
use mp_transactions::to_starknet_core_transaction::to_starknet_core_tx;
use mp_types::block::DBlockT;
use sc_client_api::backend::{Backend, StorageProvider};
use sc_client_api::BlockBackend;
use sp_blockchain::HeaderBackend;
use starknet_core::types::{FieldElement, TransactionStatus};

use crate::deoxys_backend_client::get_block_by_block_hash;
use crate::errors::StarknetRpcApiError;
use crate::utils::helpers::{block_hash_from_block_n, txs_hashes_from_block_hash};
use crate::Starknet;

/// Gets the Transaction Status, Including Mempool Status and Execution Details
///
/// This method retrieves the status of a specified transaction. It provides information on
/// whether the transaction is still in the mempool, has been executed, or dropped from the
/// mempool. The status includes both finality status and execution status of the
/// transaction.
///
/// ### Arguments
///
/// * `transaction_hash` - The hash of the transaction for which the status is requested.
///
/// ### Returns
///
/// * `transaction_status` - An object containing the transaction status details:
///   - `finality_status`: The finality status of the transaction, indicating whether it is
///     confirmed, pending, or rejected.
///   - `execution_status`: The execution status of the transaction, providing details on the
///     execution outcome if the transaction has been processed.
pub fn get_transaction_status<BE, C, H>(
    starknet: &Starknet<BE, C, H>,
    transaction_hash: FieldElement,
) -> RpcResult<TransactionStatus>
where
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    H: HasherT + Send + Sync + 'static,
{
    let substrate_block_hash = DeoxysBackend::mapping()
        .substrate_block_hash_from_transaction_hash(Felt252Wrapper(transaction_hash).into())
        .map_err(|e| {
            log::error!("Failed to get substrate block hash from transaction hash: {}", e);
            StarknetRpcApiError::InternalServerError
        })?
        .ok_or(StarknetRpcApiError::TxnHashNotFound)?;

    let starknet_block = get_block_by_block_hash(starknet.client.as_ref(), substrate_block_hash)?;
    let block_number = starknet_block.header().block_number;
    let starknet_block_hash = block_hash_from_block_n(block_number)?;

    let _starknet_tx = txs_hashes_from_block_hash(starknet_block_hash)?
        .into_iter()
        .zip(starknet_block.transactions())
        .find(|(tx_hash, _)| *tx_hash == Felt252Wrapper(transaction_hash).into())
        .map(|(_, tx)| to_starknet_core_tx(tx.clone(), transaction_hash));

    // TODO: Implement this method
    Err(StarknetRpcApiError::UnimplementedMethod.into())

    // let execution_status = {
    //     let revert_error = starknet
    //         .client
    //         .runtime_api()
    //         .get_tx_execution_outcome(substrate_block_hash,
    // Felt252Wrapper(transaction_hash).into())         .map_err(|e| {
    //             log::error!(
    //                 "Failed to get transaction execution outcome. Substrate block hash:
    // {substrate_block_hash}, \                  transaction hash: {transaction_hash}, error:
    // {e}"             );
    //             StarknetRpcApiError::InternalServerError
    //         })?;

    //     if revert_error.is_none() {
    //         TransactionExecutionStatus::Succeeded
    //     } else {
    //         TransactionExecutionStatus::Reverted
    //     }
    // };

    // Ok(TransactionStatus::AcceptedOnL2(execution_status))
}
