use {
    solana_pubkey::{
        Pubkey,
        ParsePubkeyError,
    },
    solana_transaction_status_client_types::{
        UiLoadedAddresses,
    },
    serde::{
        Deserialize, Serialize,
    },
    std::{
        str::FromStr,
    },
    log::{debug},
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
            .map(|s| {
                debug!("Parsing writable pubkey: {}", s); // Debugging
                Pubkey::from_str(s)
            })
            .collect();

        let readonly: Result<Vec<Pubkey>, _> = ui_loaded_addresses
            .readonly
            .iter()
            .map(|s| {
                debug!("Parsing readonly pubkey: {}", s); // Debugging
                Pubkey::from_str(s)
            })
            .collect();

        Ok(Self {
            writable: writable?,
            readonly: readonly?,
        })
    }
}

impl From<LoadedAddresses> for solana_message::v0::LoadedAddresses {
    fn from(addresses: LoadedAddresses) -> Self {
        Self {
            writable: addresses.writable,
            readonly: addresses.readonly,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_pubkey::Pubkey;
    use solana_transaction_status_client_types::UiLoadedAddresses;
    use std::str::FromStr;

    #[test]
    fn test_loaded_addresses_try_from_valid_ui_loaded_addresses() {
        let ui_loaded_addresses = UiLoadedAddresses {
            writable: vec![
                "11111111111111111111111111111111".to_string(),
                "22222222222222222222222222222222".to_string(),
            ],
            readonly: vec![
                "33333333333333333333333333333333".to_string(),
                "44444444444444444444444444444444".to_string(),
            ],
        };

        let loaded_addresses = LoadedAddresses::try_from(&ui_loaded_addresses);
        assert!(loaded_addresses.is_err(), "Expected error due to invalid base58 encoding");
    }

    #[test]
    fn test_loaded_addresses_try_from_invalid_ui_loaded_addresses() {
        let ui_loaded_addresses = UiLoadedAddresses {
            writable: vec!["invalid_pubkey".to_string()],
            readonly: vec!["44444444444444444444444444444444".to_string()],
        };

        let result = LoadedAddresses::try_from(&ui_loaded_addresses);
        assert!(result.is_err());
    }

    #[test]
    fn test_loaded_addresses_try_from_empty_ui_loaded_addresses() {
        let ui_loaded_addresses = UiLoadedAddresses {
            writable: vec![],
            readonly: vec![],
        };

        let loaded_addresses = LoadedAddresses::try_from(&ui_loaded_addresses).unwrap();

        assert!(loaded_addresses.writable.is_empty());
        assert!(loaded_addresses.readonly.is_empty());
    }

    #[test]
    fn test_loaded_addresses_into_solana_message_loaded_addresses() {
        let loaded_addresses = LoadedAddresses {
            writable: vec![Pubkey::new_unique()],
            readonly: vec![Pubkey::new_unique()],
        };

        let solana_loaded_addresses: solana_message::v0::LoadedAddresses = loaded_addresses.clone().into();

        assert_eq!(solana_loaded_addresses.writable, loaded_addresses.writable);
        assert_eq!(solana_loaded_addresses.readonly, loaded_addresses.readonly);
    }

    #[test]
    fn test_loaded_addresses_try_from_ui_loaded_addresses_with_correct_pubkeys() {
        let ui_loaded_addresses = UiLoadedAddresses {
            writable: vec![Pubkey::new_unique().to_string()],
            readonly: vec![Pubkey::new_unique().to_string()],
        };

        let result = LoadedAddresses::try_from(&ui_loaded_addresses);
        assert!(result.is_ok(), "Expected successful conversion");
    }

    #[test]
    fn test_loaded_addresses_try_from_ui_loaded_addresses_with_wrong_size_writable_pubkeys() {
        let ui_loaded_addresses = UiLoadedAddresses {
            writable: vec!["12345".to_string()],
            readonly: vec![Pubkey::new_unique().to_string()],
        };

        let result = LoadedAddresses::try_from(&ui_loaded_addresses);
        assert!(result.is_err(), "Expected an error due to incorrect pubkey size");
    }

    #[test]
    fn test_loaded_addresses_try_from_ui_loaded_addresses_with_wrong_size_readonly_pubkeys() {
        let ui_loaded_addresses = UiLoadedAddresses {
            writable: vec![Pubkey::new_unique().to_string()],
            readonly: vec!["67890".to_string()],
        };

        let result = LoadedAddresses::try_from(&ui_loaded_addresses);
        assert!(result.is_err(), "Expected an error due to incorrect readonly pubkey size");
    }
}


