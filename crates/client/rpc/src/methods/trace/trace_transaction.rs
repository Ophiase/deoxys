use blockifier::transaction::account_transaction::AccountTransaction;
use jsonrpsee::core::RpcResult;
use mc_db::DeoxysBackend;
use mp_felt::Felt252Wrapper;
use mp_hashers::HasherT;
use mp_transactions::TxType;
use mp_types::block::DBlockT;
use sc_client_api::{Backend, BlockBackend, StorageProvider};
use sp_blockchain::HeaderBackend;
use starknet_api::transaction::Transaction;
use starknet_core::types::TransactionTraceWithHash;
use starknet_ff::FieldElement;

use super::super::read::get_transaction_receipt::execution_infos;
use super::utils::tx_execution_infos_to_tx_trace;
use crate::deoxys_backend_client::get_block_by_block_hash;
use crate::errors::StarknetRpcApiError;
use crate::utils::execution::block_context;
use crate::utils::helpers::{block_hash_from_block_n, tx_hash_retrieve, txs_hashes_from_block_hash};
use crate::utils::transaction::blockifier_transactions;
use crate::Starknet;

pub async fn trace_transaction<BE, C, H>(
    starknet: &Starknet<BE, C, H>,
    transaction_hash: FieldElement,
) -> RpcResult<TransactionTraceWithHash>
where
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    H: HasherT + Send + Sync + 'static,
{
    let substrate_block_hash = DeoxysBackend::mapping()
        .substrate_block_hash_from_transaction_hash(Felt252Wrapper(transaction_hash).into())
        .map_err(|e| {
            log::error!("Failed to get substrate block hash from transaction hash: {}", e);
            StarknetRpcApiError::TxnHashNotFound
        })?
        .ok_or(StarknetRpcApiError::TxnHashNotFound)?;

    let starknet_block = get_block_by_block_hash(starknet.client.as_ref(), substrate_block_hash)?;
    let block_header = starknet_block.header();
    let block_number = block_header.block_number;
    let block_hash = block_hash_from_block_n(block_number)?;
    let block_context = block_context(starknet.client.as_ref(), substrate_block_hash)?;

    let block_txs_hashes = tx_hash_retrieve(txs_hashes_from_block_hash(block_hash)?);

    // retrieve the transaction index in the block with the transaction hash
    let (tx_index, _) =
        block_txs_hashes.iter().enumerate().find(|(_, hash)| *hash == &transaction_hash).ok_or_else(|| {
            log::error!("Failed to retrieve transaction index from block with hash {block_hash:?}");
            StarknetRpcApiError::InternalServerError
        })?;

    // create a vector of tuples with the transaction and its hash, up to the current transaction index
    let transaction_with_hash = starknet_block
        .transactions()
        .iter()
        .cloned()
        .zip(block_txs_hashes.iter().cloned())
        .filter(|(tx, _)| !matches!(tx, Transaction::Deploy(_)))
        .take(tx_index + 1)
        .collect();

    let transactions_blockifier = blockifier_transactions(transaction_with_hash)?;

    let last_transaction = transactions_blockifier.last().expect("There should be at least one transaction");

    let tx_type = match last_transaction {
        blockifier::transaction::transaction_execution::Transaction::AccountTransaction(account_tx) => match account_tx
        {
            AccountTransaction::Declare(_) => TxType::Declare,
            AccountTransaction::DeployAccount(_) => TxType::DeployAccount,
            AccountTransaction::Invoke(_) => TxType::Invoke,
        },
        blockifier::transaction::transaction_execution::Transaction::L1HandlerTransaction(_) => TxType::L1Handler,
    };

    let execution_infos = execution_infos(transactions_blockifier, &block_context)?;

    let trace = tx_execution_infos_to_tx_trace(tx_type, &execution_infos, block_number).unwrap();

    let tx_trace = TransactionTraceWithHash { transaction_hash, trace_root: trace };

    Ok(tx_trace)
}
