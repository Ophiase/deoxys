use jsonrpsee::core::{async_trait, RpcResult};
use mp_hashers::HasherT;
use mp_types::block::DBlockT;
use sc_client_api::backend::{Backend, StorageProvider};
use sc_client_api::BlockBackend;
use sp_blockchain::HeaderBackend;
use starknet_core::types::{
    BlockHashAndNumber, BlockId, BroadcastedTransaction, ContractClass, EventFilterWithPage, EventsPage, FeeEstimate,
    FieldElement, FunctionCall, MaybePendingBlockWithReceipts, MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs,
    MaybePendingStateUpdate, MsgFromL1, SimulationFlagForEstimateFee, SyncStatusType, Transaction,
    TransactionReceiptWithBlockInfo, TransactionStatus,
};

use super::block_hash_and_number::*;
use super::call::*;
use super::estimate_fee::*;
use super::estimate_message_fee::*;
use super::get_block_transaction_count::*;
use super::get_block_with_receipts::*;
use super::get_block_with_tx_hashes::*;
use super::get_block_with_txs::*;
use super::get_class::*;
use super::get_class_at::*;
use super::get_class_hash_at::*;
use super::get_events::*;
use super::get_nonce::*;
use super::get_state_update::*;
use super::get_storage_at::*;
use super::get_transaction_by_block_id_and_index::*;
use super::get_transaction_by_hash::*;
use super::get_transaction_receipt::*;
use super::get_transaction_status::*;
use super::syncing::*;
use crate::{Felt, Starknet, StarknetReadRpcApiServer};

#[async_trait]
impl<BE, C, H> StarknetReadRpcApiServer for Starknet<BE, C, H>
where
    BE: Backend<DBlockT> + 'static,
    C: HeaderBackend<DBlockT> + BlockBackend<DBlockT> + StorageProvider<DBlockT, BE> + 'static,
    H: HasherT + Send + Sync + 'static,
{
    fn block_number(&self) -> RpcResult<u64> {
        self.current_block_number()
    }

    fn spec_version(&self) -> RpcResult<String> {
        self.current_spec_version()
    }

    fn block_hash_and_number(&self) -> RpcResult<BlockHashAndNumber> {
        block_hash_and_number(self)
    }

    fn call(&self, request: FunctionCall, block_id: BlockId) -> RpcResult<Vec<String>> {
        call(self, request, block_id)
    }

    fn chain_id(&self) -> RpcResult<Felt> {
        self.chain_id()
    }

    fn get_block_transaction_count(&self, block_id: BlockId) -> RpcResult<u128> {
        get_block_transaction_count(self, block_id)
    }

    async fn estimate_fee(
        &self,
        request: Vec<BroadcastedTransaction>,
        simulation_flags: Vec<SimulationFlagForEstimateFee>,
        block_id: BlockId,
    ) -> RpcResult<Vec<FeeEstimate>> {
        estimate_fee(self, request, simulation_flags, block_id).await
    }

    async fn estimate_message_fee(&self, message: MsgFromL1, block_id: BlockId) -> RpcResult<FeeEstimate> {
        estimate_message_fee(self, message, block_id).await
    }

    async fn get_block_with_receipts(&self, block_id: BlockId) -> RpcResult<MaybePendingBlockWithReceipts> {
        get_block_with_receipts(self, block_id)
    }

    fn get_block_with_tx_hashes(&self, block_id: BlockId) -> RpcResult<MaybePendingBlockWithTxHashes> {
        get_block_with_tx_hashes(self, block_id)
    }

    fn get_block_with_txs(&self, block_id: BlockId) -> RpcResult<MaybePendingBlockWithTxs> {
        get_block_with_txs(self, block_id)
    }

    fn get_class_at(&self, block_id: BlockId, contract_address: FieldElement) -> RpcResult<ContractClass> {
        get_class_at(block_id, contract_address)
    }

    fn get_class_hash_at(&self, block_id: BlockId, contract_address: FieldElement) -> RpcResult<Felt> {
        get_class_hash_at(block_id, contract_address)
    }

    fn get_class(&self, block_id: BlockId, class_hash: FieldElement) -> RpcResult<ContractClass> {
        get_class(block_id, class_hash)
    }

    async fn get_events(&self, filter: EventFilterWithPage) -> RpcResult<EventsPage> {
        get_events(self, filter).await
    }

    fn get_nonce(&self, block_id: BlockId, contract_address: FieldElement) -> RpcResult<Felt> {
        get_nonce(block_id, contract_address)
    }

    fn get_storage_at(&self, contract_address: FieldElement, key: FieldElement, block_id: BlockId) -> RpcResult<Felt> {
        get_storage_at(self, contract_address, key, block_id)
    }

    fn get_transaction_by_block_id_and_index(&self, block_id: BlockId, index: u64) -> RpcResult<Transaction> {
        get_transaction_by_block_id_and_index(self, block_id, index)
    }

    fn get_transaction_by_hash(&self, transaction_hash: FieldElement) -> RpcResult<Transaction> {
        get_transaction_by_hash(self, transaction_hash)
    }

    async fn get_transaction_receipt(
        &self,
        transaction_hash: FieldElement,
    ) -> RpcResult<TransactionReceiptWithBlockInfo> {
        get_transaction_receipt(self, transaction_hash).await
    }

    fn get_transaction_status(&self, transaction_hash: FieldElement) -> RpcResult<TransactionStatus> {
        get_transaction_status(self, transaction_hash)
    }

    async fn syncing(&self) -> RpcResult<SyncStatusType> {
        syncing(self).await
    }

    fn get_state_update(&self, block_id: BlockId) -> RpcResult<MaybePendingStateUpdate> {
        get_state_update(self, block_id)
    }
}
