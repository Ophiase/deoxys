use std::collections::HashMap;

use mp_convert::field_element::FromFieldElement;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use sp_core::H256;
use starknet_api::core::{ClassHash, CompiledClassHash, ContractAddress, Nonce, PatriciaKey};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey;
use starknet_core::types::{
    ContractStorageDiffItem, DeclaredClassItem, DeployedContractItem, NonceUpdate, ReplacedClassItem, StateUpdate,
    StorageEntry,
};
use storage_handler::primitives::contract_class::{
    ClassUpdateWrapper, ContractClassData, ContractClassWrapper, StorageContractClassData,
};

use crate::mapping_db::MappingCommitment;
use crate::storage_handler::{self, DeoxysStorageError, StorageViewMut};
use crate::{DbError, DeoxysBackend};

pub async fn store_state_update(block_number: u64, state_update: StateUpdate) -> Result<(), DeoxysStorageError> {
    let state_diff = state_update.state_diff.clone();
    let nonce_map: HashMap<ContractAddress, Nonce> = state_update
        .state_diff
        .nonces
        .into_iter()
        .map(|NonceUpdate { contract_address, nonce }| {
            (
                ContractAddress(PatriciaKey(StarkFelt::new_unchecked(contract_address.to_bytes_be()))),
                Nonce(StarkFelt::new_unchecked(nonce.to_bytes_be())),
            )
        })
        .collect();

    log::debug!("💾 update state: block_number: {}", block_number);

    // Contract address to class hash and nonce update
    let fut1 = async move {
        let handler_contract_data_class = storage_handler::contract_class_hash_mut();
        let handler_contract_data_nonces = storage_handler::contract_nonces_mut();

        state_update
            .state_diff
            .deployed_contracts
            .into_iter()
            .map(|DeployedContractItem { address, class_hash }| {
                (ContractAddress::from_field_element(address), ClassHash::from_field_element(class_hash))
            })
            .try_for_each(|(contract_address, class_hash)| -> Result<(), DeoxysStorageError> {
                handler_contract_data_class.insert(contract_address, class_hash)?;
                // insert nonces for contracts that were deployed in this block and do not have a nonce
                if !nonce_map.contains_key(&contract_address) {
                    handler_contract_data_nonces.insert(contract_address, Nonce::default())?;
                }
                Ok(())
            })?;

        state_update
            .state_diff
            .replaced_classes
            .into_iter()
            .map(|ReplacedClassItem { contract_address, class_hash }| {
                (ContractAddress::from_field_element(contract_address), ClassHash::from_field_element(class_hash))
            })
            .try_for_each(|(contract_address, class_hash)| -> Result<(), DeoxysStorageError> {
                handler_contract_data_class.insert(contract_address, class_hash)?;
                Ok(())
            })?;

        // insert nonces for contracts that were not deployed or replaced in this block
        nonce_map.into_iter().for_each(|(contract_address, nonce)| {
            handler_contract_data_nonces.insert(contract_address, nonce).unwrap();
        });

        handler_contract_data_class.commit(block_number)?;
        handler_contract_data_nonces.commit(block_number)?;
        Ok(())
    };

    // Class hash to compiled class hash update
    let fut2 = async move {
        let handler_contract_class_hashes = storage_handler::contract_class_hashes_mut();

        state_update
            .state_diff
            .declared_classes
            .into_iter()
            .map(|DeclaredClassItem { class_hash, compiled_class_hash }| {
                (
                    ClassHash(StarkFelt::new_unchecked(class_hash.to_bytes_be())),
                    CompiledClassHash(StarkFelt::new_unchecked(compiled_class_hash.to_bytes_be())),
                )
            })
            .for_each(|(class_hash, compiled_class_hash)| {
                handler_contract_class_hashes.insert(class_hash, compiled_class_hash).unwrap();
            });

        handler_contract_class_hashes.commit(block_number)
    };

    // Block number to state diff update
    let fut3 = async move { storage_handler::block_state_diff().insert(block_number, state_diff) };

    let (result1, result2, result3) = tokio::join!(fut1, fut2, fut3);

    result1.and(result2).and(result3)
}

pub async fn store_class_update(block_number: u64, class_update: ClassUpdateWrapper) -> Result<(), DeoxysStorageError> {
    let handler_contract_class_data_mut = storage_handler::contract_class_data_mut();

    class_update.0.into_iter().for_each(
        |ContractClassData { hash: class_hash, contract_class: contract_class_wrapper }| {
            let ContractClassWrapper { contract: contract_class, abi, sierra_program_length, abi_length } =
                contract_class_wrapper;

            handler_contract_class_data_mut
                .insert(
                    class_hash,
                    StorageContractClassData { contract_class, abi, sierra_program_length, abi_length, block_number },
                )
                .unwrap();
        },
    );

    handler_contract_class_data_mut.commit(block_number)
}

pub async fn store_key_update(
    block_number: u64,
    storage_diffs: &[ContractStorageDiffItem],
) -> Result<(), DeoxysStorageError> {
    let handler_storage = storage_handler::contract_storage_mut();

    storage_diffs.into_par_iter().try_for_each(|ContractStorageDiffItem { address, storage_entries }| {
        let contract_address = ContractAddress::from_field_element(*address);
        storage_entries.iter().try_for_each(|StorageEntry { key, value }| -> Result<(), DeoxysStorageError> {
            let key = StorageKey::from_field_element(key);
            let value = StarkFelt::from_field_element(value);
            handler_storage.insert((contract_address, key), value)
        })
    })?;

    handler_storage.commit(block_number)?;

    Ok(())
}

pub async fn store_mapping(
    block_number: u64,
    starknet_block_hash: StarkFelt,
    substrate_block_hash: H256,
    starknet_transaction_hashes: Vec<StarkFelt>,
) -> Result<(), DbError> {
    let mapping_commitment = MappingCommitment {
        block_number,
        block_hash: substrate_block_hash,
        starknet_block_hash,
        starknet_transaction_hashes,
    };

    DeoxysBackend::mapping().write_hashes(mapping_commitment)
}
