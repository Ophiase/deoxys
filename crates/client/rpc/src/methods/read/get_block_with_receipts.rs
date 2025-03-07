use jsonrpsee::core::RpcResult;
use mp_hashers::HasherT;
use mp_transactions::to_starknet_core_transaction::to_starknet_core_tx;
use mp_types::block::DBlockT;
use sc_client_api::backend::{Backend, StorageProvider};
use sc_client_api::BlockBackend;
use sp_blockchain::HeaderBackend;
use starknet_api::transaction::Transaction;
use starknet_core::types::{
    BlockId, BlockTag, BlockWithReceipts, MaybePendingBlockWithReceipts, PendingBlockWithReceipts, TransactionReceipt,
    TransactionWithReceipt,
};

use super::get_transaction_receipt::receipt;
use crate::deoxys_backend_client::get_block_by_block_hash;
use crate::errors::StarknetRpcApiError;
use crate::utils::block::{
    l1_da_mode, l1_data_gas_price, l1_gas_price, new_root, parent_hash, sequencer_address, starknet_version, timestamp,
};
use crate::utils::execution::{block_context, re_execute_transactions};
use crate::utils::helpers::{block_hash_from_block_n, status, tx_hash_retrieve, txs_hashes_from_block_hash};
use crate::utils::transaction::blockifier_transactions;
use crate::Starknet;

pub fn get_block_with_receipts<BE, C, H>(
    starknet: &Starknet<BE, C, H>,
    block_id: BlockId,
) -> RpcResult<MaybePendingBlockWithReceipts>
where
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    H: HasherT + Send + Sync + 'static,
{
    let substrate_block_hash = starknet.substrate_block_hash_from_starknet_block(block_id)?;
    let block = get_block_by_block_hash(starknet.client.as_ref(), substrate_block_hash)?;
    let block_header = block.header();
    let block_number = block_header.block_number;
    let block_hash = block_hash_from_block_n(block_number)?;

    let block_context = block_context(starknet.client.as_ref(), substrate_block_hash)?;

    let block_txs_hashes = tx_hash_retrieve(txs_hashes_from_block_hash(block_hash)?);

    // create a vector of transactions with their corresponding hashes without deploy transactions,
    // blockifier does not support deploy transactions
    let transaction_with_hash: Vec<_> = block
        .transactions()
        .iter()
        .cloned()
        .zip(block_txs_hashes)
        .filter(|(tx, _)| !matches!(tx, Transaction::Deploy(_)))
        .collect();

    let transactions_blockifier = blockifier_transactions(transaction_with_hash.clone())?;

    let execution_infos = re_execute_transactions(vec![], transactions_blockifier, &block_context).map_err(|e| {
        log::error!("Failed to re-execute transactions: '{e}'");
        StarknetRpcApiError::InternalServerError
    })?;

    let transactions_core: Vec<_> = transaction_with_hash
        .iter()
        .cloned()
        .map(|(transaction, hash)| to_starknet_core_tx(transaction, hash))
        .collect();

    let receipts: Vec<TransactionReceipt> = execution_infos
        .iter()
        .zip(transaction_with_hash)
        .map(|(execution_info, (transaction, transaction_hash))| {
            receipt(&transaction, execution_info, transaction_hash, block_number)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let transactions_with_receipts = transactions_core
        .into_iter()
        .zip(receipts)
        .map(|(transaction, receipt)| TransactionWithReceipt { transaction, receipt })
        .collect();

    let is_pending = matches!(block_id, BlockId::Tag(BlockTag::Pending));

    let starknet_block = get_block_by_block_hash(starknet.client.as_ref(), substrate_block_hash).map_err(|e| {
        log::error!("Failed to get block for block hash {substrate_block_hash}: '{e}'");
        StarknetRpcApiError::InternalServerError
    })?;

    if is_pending {
        let pending_block_with_receipts = PendingBlockWithReceipts {
            transactions: transactions_with_receipts,
            parent_hash: parent_hash(&starknet_block),
            timestamp: timestamp(&starknet_block),
            sequencer_address: sequencer_address(&starknet_block),
            l1_gas_price: l1_gas_price(&starknet_block),
            l1_data_gas_price: l1_data_gas_price(&starknet_block),
            l1_da_mode: l1_da_mode(&starknet_block),
            starknet_version: starknet_version(&starknet_block),
        };

        let pending_block = MaybePendingBlockWithReceipts::PendingBlock(pending_block_with_receipts);
        Ok(pending_block)
    } else {
        let block_with_receipts = BlockWithReceipts {
            status: status(starknet_block.header().block_number),
            block_hash,
            parent_hash: parent_hash(&starknet_block),
            block_number,
            new_root: new_root(&starknet_block),
            timestamp: timestamp(&starknet_block),
            sequencer_address: sequencer_address(&starknet_block),
            l1_gas_price: l1_gas_price(&starknet_block),
            l1_data_gas_price: l1_data_gas_price(&starknet_block),
            l1_da_mode: l1_da_mode(&starknet_block),
            starknet_version: starknet_version(&starknet_block),
            transactions: transactions_with_receipts,
        };
        Ok(MaybePendingBlockWithReceipts::Block(block_with_receipts))
    }
}
