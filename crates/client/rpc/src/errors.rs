use jsonrpsee::types::error::{CallError, ErrorObject};
use mc_db::storage_handler::DeoxysStorageError;
use mc_db::DbError;
use pallet_starknet_runtime_api::StarknetTransactionExecutionError;
use starknet_core::types::StarknetError;

// Comes from the RPC Spec:
// https://github.com/starkware-libs/starknet-specs/blob/0e859ff905795f789f1dfd6f7340cdaf5015acc8/api/starknet_write_api.json#L227
#[derive(thiserror::Error, Debug)]
pub enum StarknetRpcApiError {
    #[error("Failed to write transaction")]
    FailedToReceiveTxn,
    #[error("Contract not found")]
    ContractNotFound,
    #[error("Block not found")]
    BlockNotFound,
    #[error("Invalid transaction hash")]
    InvalidTxnHash,
    #[error("Invalid tblock hash")]
    InvalidBlockHash,
    #[error("Invalid transaction index in a block")]
    InvalidTxnIndex,
    #[error("Class hash not found")]
    ClassHashNotFound,
    #[error("Transaction hash not found")]
    TxnHashNotFound,
    #[error("Requested page size is too big")]
    PageSizeTooBig,
    #[error("There are no blocks")]
    NoBlocks,
    #[error("The supplied continuation token is invalid or unknown")]
    InvalidContinuationToken,
    #[error("Too many keys provided in a filter")]
    TooManyKeysInFilter,
    #[error("Failed to fetch pending transactions")]
    FailedToFetchPendingTransactions,
    #[error("Contract error")]
    ContractError,
    #[error("Transaction execution error")]
    TxnExecutionError,
    #[error("Invalid contract class")]
    InvalidContractClass,
    #[error("Class already declared")]
    ClassAlreadyDeclared,
    #[error("Invalid transaction nonce")]
    InvalidTxnNonce,
    #[error("Max fee is smaller than the minimal transaction cost (validation plus fee transfer)")]
    InsufficientMaxFee,
    #[error("Account balance is smaller than the transaction's max_fee")]
    InsufficientAccountBalance,
    #[error("Account validation failed")]
    ValidationFailure,
    #[error("Compilation failed")]
    CompilationFailed,
    #[error("Contract class size is too large")]
    ContractClassSizeTooLarge,
    #[error("Sender address is not an account contract")]
    NonAccount,
    #[error("A transaction with the same hash already exists in the mempool")]
    DuplicateTxn,
    #[error("The compiled class hash did not match the one supplied in the transaction")]
    CompiledClassHashMismatch,
    #[error("The transaction version is not supported")]
    UnsupportedTxnVersion,
    #[error("The contract class version is not supported")]
    UnsupportedContractClassVersion,
    #[error("An unexpected error occurred")]
    ErrUnexpectedError { data: String },
    #[error("Internal server error")]
    InternalServerError,
    #[error("Unimplemented method")]
    UnimplementedMethod,
    #[error("Too many storage keys requested")]
    ProofLimitExceeded,
}

impl From<&StarknetRpcApiError> for i32 {
    fn from(err: &StarknetRpcApiError) -> Self {
        match err {
            StarknetRpcApiError::FailedToReceiveTxn => 1,
            StarknetRpcApiError::ContractNotFound => 20,
            StarknetRpcApiError::BlockNotFound => 24,
            StarknetRpcApiError::InvalidTxnHash => 25,
            StarknetRpcApiError::InvalidBlockHash => 26,
            StarknetRpcApiError::InvalidTxnIndex => 27,
            StarknetRpcApiError::ClassHashNotFound => 28,
            StarknetRpcApiError::TxnHashNotFound => 29,
            StarknetRpcApiError::PageSizeTooBig => 31,
            StarknetRpcApiError::NoBlocks => 32,
            StarknetRpcApiError::InvalidContinuationToken => 33,
            StarknetRpcApiError::TooManyKeysInFilter => 34,
            StarknetRpcApiError::FailedToFetchPendingTransactions => 38,
            StarknetRpcApiError::ContractError => 40,
            StarknetRpcApiError::TxnExecutionError => 41,
            StarknetRpcApiError::InvalidContractClass => 50,
            StarknetRpcApiError::ClassAlreadyDeclared => 51,
            StarknetRpcApiError::InvalidTxnNonce => 52,
            StarknetRpcApiError::InsufficientMaxFee => 53,
            StarknetRpcApiError::InsufficientAccountBalance => 54,
            StarknetRpcApiError::ValidationFailure => 55,
            StarknetRpcApiError::CompilationFailed => 56,
            StarknetRpcApiError::ContractClassSizeTooLarge => 57,
            StarknetRpcApiError::NonAccount => 58,
            StarknetRpcApiError::DuplicateTxn => 59,
            StarknetRpcApiError::CompiledClassHashMismatch => 60,
            StarknetRpcApiError::UnsupportedTxnVersion => 61,
            StarknetRpcApiError::UnsupportedContractClassVersion => 62,
            StarknetRpcApiError::ErrUnexpectedError { data: _ } => 63,
            StarknetRpcApiError::InternalServerError => 500,
            StarknetRpcApiError::UnimplementedMethod => 501,
            StarknetRpcApiError::ProofLimitExceeded => 10000,
        }
    }
}

impl StarknetRpcApiError {
    pub fn data(&self) -> Option<String> {
        match self {
            StarknetRpcApiError::ErrUnexpectedError { data } => Some(data.clone()),
            _ => None,
        }
    }
}

impl From<StarknetTransactionExecutionError> for StarknetRpcApiError {
    fn from(err: StarknetTransactionExecutionError) -> Self {
        match err {
            StarknetTransactionExecutionError::ContractNotFound => StarknetRpcApiError::ContractNotFound,
            StarknetTransactionExecutionError::ClassAlreadyDeclared => StarknetRpcApiError::ClassAlreadyDeclared,
            StarknetTransactionExecutionError::ClassHashNotFound => StarknetRpcApiError::ClassHashNotFound,
            StarknetTransactionExecutionError::InvalidContractClass => StarknetRpcApiError::InvalidContractClass,
            StarknetTransactionExecutionError::ContractError => StarknetRpcApiError::ContractError,
        }
    }
}

impl From<StarknetRpcApiError> for jsonrpsee::core::Error {
    fn from(err: StarknetRpcApiError) -> Self {
        jsonrpsee::core::Error::Call(CallError::Custom(ErrorObject::owned((&err).into(), err.to_string(), err.data())))
    }
}

impl From<StarknetError> for StarknetRpcApiError {
    fn from(err: StarknetError) -> Self {
        match err {
            StarknetError::FailedToReceiveTransaction => StarknetRpcApiError::FailedToReceiveTxn,
            StarknetError::ContractNotFound => StarknetRpcApiError::ContractNotFound,
            StarknetError::BlockNotFound => StarknetRpcApiError::BlockNotFound,
            StarknetError::InvalidTransactionIndex => StarknetRpcApiError::InvalidTxnIndex,
            StarknetError::ClassHashNotFound => StarknetRpcApiError::ClassHashNotFound,
            StarknetError::TransactionHashNotFound => StarknetRpcApiError::TxnHashNotFound,
            StarknetError::PageSizeTooBig => StarknetRpcApiError::PageSizeTooBig,
            StarknetError::NoBlocks => StarknetRpcApiError::NoBlocks,
            StarknetError::InvalidContinuationToken => StarknetRpcApiError::InvalidContinuationToken,
            StarknetError::TooManyKeysInFilter => StarknetRpcApiError::TooManyKeysInFilter,
            StarknetError::ContractError(_) => StarknetRpcApiError::ContractError,
            StarknetError::ClassAlreadyDeclared => StarknetRpcApiError::ClassAlreadyDeclared,
            StarknetError::InvalidTransactionNonce => StarknetRpcApiError::InvalidTxnNonce,
            StarknetError::InsufficientMaxFee => StarknetRpcApiError::InsufficientMaxFee,
            StarknetError::InsufficientAccountBalance => StarknetRpcApiError::InsufficientAccountBalance,
            StarknetError::ValidationFailure(_) => StarknetRpcApiError::ValidationFailure,
            StarknetError::CompilationFailed => StarknetRpcApiError::CompilationFailed,
            StarknetError::ContractClassSizeIsTooLarge => StarknetRpcApiError::ContractClassSizeTooLarge,
            StarknetError::NonAccount => StarknetRpcApiError::NonAccount,
            StarknetError::DuplicateTx => StarknetRpcApiError::DuplicateTxn,
            StarknetError::CompiledClassHashMismatch => StarknetRpcApiError::CompiledClassHashMismatch,
            StarknetError::UnsupportedTxVersion => StarknetRpcApiError::UnsupportedTxnVersion,
            StarknetError::UnsupportedContractClassVersion => StarknetRpcApiError::UnsupportedContractClassVersion,
            StarknetError::UnexpectedError(data) => StarknetRpcApiError::ErrUnexpectedError { data },
            StarknetError::NoTraceAvailable(_) => StarknetRpcApiError::InternalServerError,
            StarknetError::TransactionExecutionError(_) => StarknetRpcApiError::TxnExecutionError,
        }
    }
}

impl From<DeoxysStorageError> for StarknetRpcApiError {
    fn from(_: DeoxysStorageError) -> Self {
        StarknetRpcApiError::ErrUnexpectedError { data: "DB error".to_string() }
    }
}

impl From<DbError> for StarknetRpcApiError {
    fn from(_: DbError) -> Self {
        StarknetRpcApiError::ErrUnexpectedError { data: "DB error".to_string() }
    }
}
