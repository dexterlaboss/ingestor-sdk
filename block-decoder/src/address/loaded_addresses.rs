use {
    solana_sdk::{
        pubkey::{Pubkey, ParsePubkeyError},
    },
    solana_transaction_status::{
        UiLoadedAddresses,
    },
    serde::{
        Deserialize, Serialize,
    },
    std::{
        str::FromStr,
    },
};

/// Collection of addresses loaded from on-chain lookup tables, split
/// by readonly and writable.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedAddresses {
    /// List of addresses for writable loaded accounts
    pub writable: Vec<Pubkey>,
    /// List of addresses for read-only loaded accounts
    pub readonly: Vec<Pubkey>,
}

impl TryFrom<&UiLoadedAddresses> for LoadedAddresses {
    type Error = ParsePubkeyError;

    fn try_from(ui_loaded_addresses: &UiLoadedAddresses) -> Result<Self, Self::Error> {
        let writable: Result<Vec<Pubkey>, _> = ui_loaded_addresses
            .writable
            .iter()
            .map(|s| Pubkey::from_str(s))
            .collect();

        let readonly: Result<Vec<Pubkey>, _> = ui_loaded_addresses
            .readonly
            .iter()
            .map(|s| Pubkey::from_str(s))
            .collect();

        Ok(Self {
            writable: writable?,
            readonly: readonly?,
        })
    }
}

impl From<LoadedAddresses> for solana_sdk::message::v0::LoadedAddresses {
    fn from(addresses: LoadedAddresses) -> Self {
        Self {
            writable: addresses.writable,
            readonly: addresses.readonly,
        }
    }
}