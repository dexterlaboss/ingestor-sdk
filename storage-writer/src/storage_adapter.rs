use {
    crate::{
        error::Error,
    },
    async_trait::async_trait,
    // solana_sdk::{
    //     clock::{
    //         Slot,
    //     },
    //     pubkey::Pubkey,
    // },
    solana_clock::{
        Slot,
    },
    solana_pubkey::{
        Pubkey,
    },
    solana_transaction_status::{
        VersionedConfirmedBlock,
    },
    std::{
        boxed::Box,
    },
};

pub type Result<T> = std::result::Result<T, Error>;


#[async_trait]
pub trait LedgerStorageAdapter: Send + Sync {
    async fn upload_confirmed_block(
        &self,
        slot: Slot,
        confirmed_block: VersionedConfirmedBlock,
    ) -> Result<()>;

    fn should_include_in_tx_full(&self, address: &Pubkey) -> bool;
    fn should_include_in_tx_by_addr(&self, address: &Pubkey) -> bool;

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter>;
}
