#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate alloc;

mod merkle_patricia_tree;

use alloc::vec::Vec;

use bitvec::vec::BitVec;
use merkle_patricia_tree::merkle_tree::{MerkleTree, NodesMapping, ProofNode};
use merkle_patricia_tree::ref_merkle_tree::RefMerkleTree;
use mp_felt::Felt252Wrapper;
use mp_hashers::HasherT;
use mp_transactions::compute_hash::ComputeTransactionHash;
use mp_transactions::Transaction;
use starknet_api::transaction::Event;
use starknet_crypto::FieldElement;

/// Hash of the StateCommitment tree
pub type StateCommitment = Felt252Wrapper;

/// Hash of the leaf of the ClassCommitment tree
pub type ClassCommitmentLeafHash = Felt252Wrapper;

/// A Patricia Merkle tree with height 64 used to compute transaction and event commitments.
///
/// According to the [documentation](https://docs.starknet.io/documentation/architecture_and_concepts/Blocks/header/)
/// the commitment trees are of height 64, because the key used is the 64 bit representation
/// of the index of the transaction / event within the block.
///
/// The tree height is 64 in our case since our set operation takes u64 index values.
struct CommitmentTree<H: HasherT> {
    tree: RefMerkleTree<H>,
}

impl<H: HasherT> Default for CommitmentTree<H> {
    fn default() -> Self {
        Self { tree: RefMerkleTree::empty() }
    }
}

impl<H: HasherT> CommitmentTree<H> {
    /// Sets the value of a key in the merkle tree.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to set.
    /// * `value` - The value to set.
    pub fn set(&mut self, index: u64, value: FieldElement) {
        let key = index.to_be_bytes();
        self.tree.set(&BitVec::from_vec(key.to_vec()), Felt252Wrapper(value))
    }

    /// Get the merkle root of the tree.
    pub fn commit(&mut self) -> Felt252Wrapper {
        self.tree.commit()
    }
}

/// A Patricia Merkle tree with height 251 used to compute contract and class tree commitments.
///
/// According to the [documentation](https://docs.starknet.io/documentation/architecture_and_concepts/State/starknet-state/)
/// the commitment trees are of height 251, because the key used is a Field Element.
///
/// The tree height is 251 in our case since our set operation takes Fieldelement index values.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "parity-scale-codec", derive(parity_scale_codec::Encode, parity_scale_codec::Decode))]
#[cfg_attr(feature = "scale-info", derive(scale_info::TypeInfo))]
pub struct StateCommitmentTree<H: HasherT> {
    tree: MerkleTree<H>,
}

impl<H: HasherT> Default for StateCommitmentTree<H> {
    fn default() -> Self {
        Self { tree: MerkleTree::empty() }
    }
}

impl<H: HasherT> StateCommitmentTree<H> {
    /// Sets the value of a key in the merkle tree.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the value to set.
    /// * `value` - The value to set.
    pub fn set(&mut self, index: Felt252Wrapper, value: Felt252Wrapper) {
        let key = &index.0.to_bytes_be()[..31];
        self.tree.set(&BitVec::from_vec(key.to_vec()), value)
    }

    /// Get the merkle root of the tree.
    pub fn commit(&mut self) -> Felt252Wrapper {
        self.tree.commit()
    }

    /// Generates a proof for `key`. See [`MerkleTree::get_proof`].
    pub fn get_proof(&self, key: Felt252Wrapper) -> Vec<ProofNode> {
        let key = &key.0.to_bytes_be()[..31];
        self.tree.get_proof(&BitVec::from_vec(key.to_vec()))
    }

    /// Returns a leaf of the tree stored at key `key`
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the value to retrieve.
    ///
    /// # Returns
    ///
    /// `Some(value)` - Value stored at the given key.
    pub fn get(&self, key: Felt252Wrapper) -> Option<Felt252Wrapper> {
        let key = &key.0.to_bytes_be()[..31];
        self.tree.get(&BitVec::from_vec(key.to_vec()))
    }

    /// Returns the tree's nodes
    pub fn nodes(&self) -> NodesMapping {
        NodesMapping(self.tree.nodes())
    }

    /// Loads tree from root
    pub fn load(root: Felt252Wrapper) -> Self {
        let merkle_tree = MerkleTree::new(root);
        Self { tree: merkle_tree }
    }
}

/// Calculate state commitment hash value.
///
/// The state commitment is the digest that uniquely (up to hash collisions) encodes the state.
/// It combines the roots of two binary Merkle-Patricia trees of height 251.
///
/// # Arguments
///
/// * `contracts_tree_root` - The root of the contracts tree.
/// * `classes_tree_root` - The root of the classes tree.
///
/// # Returns
///
/// The state commitment as a `StateCommitment`.
pub fn calculate_state_commitment<H: HasherT>(
    contracts_tree_root: Felt252Wrapper,
    classes_tree_root: Felt252Wrapper,
) -> StateCommitment
where
    H: HasherT,
{
    let starknet_state_prefix = Felt252Wrapper::try_from("STARKNET_STATE_V0".as_bytes()).unwrap();

    let state_commitment_hash =
        H::compute_hash_on_elements(&[starknet_state_prefix.0, contracts_tree_root.0, classes_tree_root.0]);

    state_commitment_hash.into()
}

/// Calculate the transaction commitment, the event commitment and the event count.
///
/// # Arguments
///
/// * `transactions` - The transactions of the block
///
/// # Returns
///
/// The transaction commitment, the event commitment and the event count.
pub fn calculate_commitments<H: HasherT>(
    transactions: &[Transaction],
    events: &[Event],
    chain_id: Felt252Wrapper,
    block_number: u64,
) -> (Felt252Wrapper, Felt252Wrapper) {
    (
        calculate_transaction_commitment::<H>(transactions, chain_id, block_number),
        calculate_event_commitment::<H>(events),
    )
}

/// Calculate transaction commitment hash value.
///
/// The transaction commitment is the root of the Patricia Merkle tree with height 64
/// constructed by adding the (transaction_index, transaction_hash_with_signature)
/// key-value pairs to the tree and computing the root hash.
///
/// # Arguments
///
/// * `transactions` - The transactions to get the root from.
///
/// # Returns
///
/// The merkle root of the merkle tree built from the transactions.
pub fn calculate_transaction_commitment<H: HasherT>(
    transactions: &[Transaction],
    chain_id: Felt252Wrapper,
    block_number: u64,
) -> Felt252Wrapper {
    let mut tree = CommitmentTree::<H>::default();

    transactions.iter().enumerate().for_each(|(idx, tx)| {
        let idx: u64 = idx.try_into().expect("too many transactions while calculating commitment");
        let final_hash = calculate_transaction_hash_with_signature::<H>(tx, chain_id, block_number);
        tree.set(idx, final_hash);
    });
    tree.commit()
}

/// Calculate event commitment hash value.
///
/// The event commitment is the root of the Patricia Merkle tree with height 64
/// constructed by adding the event hash
/// (see https://docs.starknet.io/documentation/architecture_and_concepts/Events/starknet-events/#event_hash)
/// to the tree and computing the root hash.
///
/// # Arguments
///
/// * `events` - The events to calculate the commitment from.
///
/// # Returns
///
/// The merkle root of the merkle tree built from the events.
pub(crate) fn calculate_event_commitment<H: HasherT>(events: &[Event]) -> Felt252Wrapper {
    let mut tree = CommitmentTree::<H>::default();
    events.iter().enumerate().for_each(|(id, event)| {
        let final_hash = calculate_event_hash::<H>(event);
        tree.set(id as u64, final_hash);
    });
    tree.commit()
}

/// Calculate class commitment tree leaf hash value.
///
/// See: <https://docs.starknet.io/documentation/architecture_and_concepts/State/starknet-state/#classes_tree>
///
/// # Arguments
///
/// * `compiled_class_hash` - The hash of the compiled class.
///
/// # Returns
///
/// The hash of the class commitment tree leaf.
pub fn calculate_class_commitment_leaf_hash<H: HasherT>(
    compiled_class_hash: Felt252Wrapper,
) -> ClassCommitmentLeafHash {
    let contract_class_hash_version = Felt252Wrapper::try_from("CONTRACT_CLASS_LEAF_V0".as_bytes()).unwrap(); // Unwrap safu

    let hash = H::compute_hash_on_elements(&[contract_class_hash_version.0, compiled_class_hash.0]);

    hash.into()
}

/// Calculate class commitment tree root hash value.
///
/// The classes tree encodes the information about the existing classes in the state of Starknet.
/// It maps (Cairo 1.0) class hashes to their compiled class hashes
///
/// # Arguments
///
/// * `classes` - The classes to get the root from.
///
/// # Returns
///
/// The merkle root of the merkle tree built from the classes.
pub fn calculate_class_commitment_tree_root_hash<H: HasherT>(class_hashes: &[Felt252Wrapper]) -> Felt252Wrapper {
    let mut tree = StateCommitmentTree::<H>::default();
    class_hashes.iter().for_each(|class_hash| {
        let final_hash = calculate_class_commitment_leaf_hash::<H>(*class_hash);
        tree.set(*class_hash, final_hash);
    });
    tree.commit()
}

/// Calculates the contract state hash from its preimage.
///
/// # Arguments
///
/// * `hash` - The hash of the contract definition.
/// * `root` - The root of root of another Merkle-Patricia tree of height 251 that is constructed
///   from the contract’s storage.
/// * `nonce` - The current nonce of the contract.
///
/// # Returns
///
/// The contract state hash.
pub fn calculate_contract_state_hash<H: HasherT>(
    hash: Felt252Wrapper,
    root: Felt252Wrapper,
    nonce: Felt252Wrapper,
) -> Felt252Wrapper {
    // Define the constant for the contract state hash version, ensure this aligns with StarkNet
    // specifications.
    const CONTRACT_STATE_HASH_VERSION: Felt252Wrapper = Felt252Wrapper::ZERO;

    // First hash: Combine class_hash and storage_root.
    let class_storage_hash = H::compute_hash_on_elements(&[hash.0, root.0]);
    let nonce_hash = H::compute_hash_on_elements(&[class_storage_hash, nonce.0]);
    let contract_state_hash = H::compute_hash_on_elements(&[nonce_hash, CONTRACT_STATE_HASH_VERSION.0]);

    contract_state_hash.into()
}

/// Compute the combined hash of the transaction hash and the signature.
///
/// Since the transaction hash doesn't take the signature values as its input
/// computing the transaction commitent uses a hash value that combines
/// the transaction hash with the array of signature values.
///
/// # Arguments
///
/// * `tx` - The transaction to compute the hash of.
///
/// # Returns
///
/// The transaction hash with signature.
pub fn calculate_transaction_hash_with_signature<H: HasherT>(
    tx: &Transaction,
    chain_id: Felt252Wrapper,
    block_number: u64,
) -> FieldElement
where
    H: HasherT,
{
    let include_signature = block_number >= 61394;

    let signature_hash = if matches!(tx, Transaction::Invoke(_)) || include_signature {
        // Include signatures for Invoke transactions or for all transactions
        // starting from block 61394
        H::compute_hash_on_elements(
            &tx.signature().iter().map(|elt| FieldElement::from(*elt)).collect::<Vec<FieldElement>>(),
        )
    } else {
        // Before block 61394, and for non-Invoke transactions, signatures are not included
        H::compute_hash_on_elements(&[])
    };

    let transaction_hashes =
        H::hash_elements(FieldElement::from(tx.compute_hash::<H>(chain_id, false, Some(block_number))), signature_hash);

    transaction_hashes
}

/// Calculate the hash of an event.
///
/// See the [documentation](https://docs.starknet.io/documentation/architecture_and_concepts/Events/starknet-events/#event_hash)
/// for details.
pub fn calculate_event_hash<H: HasherT>(event: &Event) -> FieldElement {
    let keys_hash = H::compute_hash_on_elements(
        &event.content.keys.iter().map(|key| FieldElement::from(Felt252Wrapper::from(key.0))).collect::<Vec<FieldElement>>(),
    );
    let data_hash = H::compute_hash_on_elements(
        &event.content.data.0.iter().map(|data| FieldElement::from(Felt252Wrapper::from(*data))).collect::<Vec<FieldElement>>(),
    );
    let from_address = FieldElement::from(Felt252Wrapper::from(event.from_address.0.0));
    H::compute_hash_on_elements(&[from_address, keys_hash, data_hash])
}

#[cfg(test)]
mod tests;
