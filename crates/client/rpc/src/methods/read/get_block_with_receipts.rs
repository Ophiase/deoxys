use jsonrpsee::core::RpcResult;
use mc_genesis_data_provider::GenesisProvider;
use mp_hashers::HasherT;
use mp_types::block::{self, DBlockT};
use pallet_starknet_runtime_api::{ConvertTransactionRuntimeApi, StarknetRuntimeApi};
use sc_client_api::backend::{Backend, StorageProvider};
use sc_client_api::BlockBackend;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use starknet_core::types::{BlockId, BlockTag, BlockWithReceipts, MaybePendingBlockWithReceipts};

use crate::errors::StarknetRpcApiError;
use crate::Starknet;
use super::get_transaction_receipt::get_transaction_receipt_finalized;
use crate::utils::get_block_by_block_hash;

//TODO: Implem pending block
pub fn get_block_with_receipts<A, BE, G, C, P, H>(
    starknet: &Starknet<A, BE, G, C, P, H>,
    block_id: BlockId,
) -> RpcResult<MaybePendingBlockWithReceipts>
where
    A: ChainApi<Block = DBlockT> + 'static,
    P: TransactionPool<Block = DBlockT> + 'static,
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    C: ProvideRuntimeApi<DBlockT>,
    C::Api: StarknetRuntimeApi<DBlockT> + ConvertTransactionRuntimeApi<DBlockT>,
    G: GenesisProvider + Send + Sync + 'static,
    H: HasherT + Send + Sync + 'static,
{
    let chain_id = starknet.chain_id()?;
    let substrate_block_hash = starknet.substrate_block_hash_from_starknet_block(block_id).map_err(|e| {
        log::error!("'{e}'");
        StarknetRpcApiError::BlockNotFound
    })?;

    let starknet_block = get_block_by_block_hash(starknet.client.as_ref(), substrate_block_hash).map_err(|e| {
        log::error!("Failed to get block for block hash {substrate_block_hash}: '{e}'");
        StarknetRpcApiError::InternalServerError
    })?;
    let chain_id = Felt252Wrapper(starknet.chain_id()?.0);

    let transactions_with_receipts = starknet_block.transactions().iter().map(|tx| {
        let transaction_hash = tx.hash();
        let transaction_receipt = get_transaction_receipt_finalized(client, chain_id, substrate_block_hash, transaction_hash).unwrap();
        transaction_receipt.0
    }).collect();

    let block_context = starknet
        .client
        .runtime_api()
        .get_block_context(runtime_api_at_param__)
        .map_err(|e| {
            log::error!("Request parameters error: {e}");
            StarknetRpcApiError::InternalServerError
        })?;

    let starknet_version = starknet.current_spec_version().map_err(|e| {
        log::error!("Failed to get current spec version: {e}");
        StarknetRpcApiError::InternalServerError
    })?;

    let block_with_receipts = MaybePendingBlockWithReceipts(BlockWithReceipts) {
        status: block_context.block_info,
        block_hash: ,
        parent_hash: starknet_block.header().parent_block_hash,
        block_number: starknet_block.header().block_number ,
        new_root: starknet_block.header().global_state_root,
        timestamp: starknet_block.header().block_timestamp,
        sequencer_address: starknet_block.header().global_state_root,
        l1_gas_price: starknet_block.header().l1_gas_price.eth_l1_gas_price, ,
        l1_data_gas_price: starknet_block.header().l1_gas_price.eth_l1_data_gas_price,
        l1_da_mode: ,
        starknet_version: starknet_version,
        transactions: transactions_with_receipts,
    };
    
    Ok(block_with_receipts)
}