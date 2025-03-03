use {
    crate::{
        error::Error,
    },
    async_trait::async_trait,
    solana_clock::{
        Slot,
    },
    solana_pubkey::{
        Pubkey,
    },
    solana_signature::{
        Signature,
    },
    solana_transaction_status::{
        ConfirmedBlock,
        ConfirmedTransactionStatusWithSignature,
        ConfirmedTransactionWithStatusMeta,
    },
    solana_transaction_status_client_types::{
        TransactionStatus,
    },
    std::{
        boxed::Box,
    },
};

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait LedgerStorageAdapter: Send + Sync {
    async fn get_first_available_block(&self) -> Result<Option<Slot>>;

    async fn get_confirmed_blocks(&self, start_slot: Slot, limit: usize) -> Result<Vec<Slot>>;

    async fn get_confirmed_block(&self, slot: Slot) -> Result<ConfirmedBlock>;

    async fn get_signature_status(&self, signature: &Signature) -> Result<TransactionStatus>;

    async fn get_full_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<ConfirmedTransactionWithStatusMeta>>;

    async fn get_confirmed_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<ConfirmedTransactionWithStatusMeta>>;

    async fn get_confirmed_signatures_for_address(
        &self,
        address: &Pubkey,
        before_signature: Option<&Signature>,
        until_signature: Option<&Signature>,
        limit: usize,
    ) -> Result<Vec<(ConfirmedTransactionStatusWithSignature, u32)>>;

    async fn get_latest_stored_slot(&self) -> Result<Slot>;

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter>;
}
