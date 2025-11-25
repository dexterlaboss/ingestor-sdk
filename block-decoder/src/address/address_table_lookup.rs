
use {
    crate::{
        errors::{
            decode_error::DecodeError,
        }
    },
    solana_short_vec as short_vec,
    solana_pubkey::{Pubkey, ParsePubkeyError},
    solana_transaction_status_client_types::{
        UiAddressTableLookup,
    },
    serde::{
        Deserialize, Serialize,
    },
    std::{
        str::FromStr,
    },
};

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MessageAddressTableLookup {
    /// Address lookup table account key
    pub account_key: Pubkey,
    /// List of indexes used to load writable account addresses
    #[serde(with = "short_vec")]
    pub writable_indexes: Vec<u8>,
    /// List of indexes used to load readonly account addresses
    #[serde(with = "short_vec")]
    pub readonly_indexes: Vec<u8>,
}

impl TryFrom<&UiAddressTableLookup> for MessageAddressTableLookup {
    type Error = DecodeError;

    fn try_from(lookup: &UiAddressTableLookup) -> Result<Self, Self::Error> {
        let account_key = Pubkey::from_str(&lookup.account_key)
            .map_err(|_| DecodeError::ParsePubkeyFailed(ParsePubkeyError::Invalid))?;
        Ok(Self {
            account_key,
            writable_indexes: lookup.writable_indexes.clone(),
            readonly_indexes: lookup.readonly_indexes.clone(),
        })
    }
}

impl From<MessageAddressTableLookup> for solana_message::v0::MessageAddressTableLookup {
    fn from(lookup: MessageAddressTableLookup) -> Self {
        Self {
            account_key: lookup.account_key,
            writable_indexes: lookup.writable_indexes,
            readonly_indexes: lookup.readonly_indexes,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use solana_message::v0::MessageAddressTableLookup as SolanaMessageAddressTableLookup;
    use solana_transaction_status_client_types::UiAddressTableLookup;
    use serde_json::{self, Value};

    #[test]
    fn test_try_from_ui_address_table_lookup_success() {
        let ui_lookup = UiAddressTableLookup {
            account_key: "11111111111111111111111111111111".to_string(),
            writable_indexes: vec![1, 2, 3],
            readonly_indexes: vec![4, 5, 6],
        };

        let lookup = MessageAddressTableLookup::try_from(&ui_lookup).unwrap();
        assert_eq!(
            lookup.account_key,
            Pubkey::from_str("11111111111111111111111111111111").unwrap()
        );
        assert_eq!(lookup.writable_indexes, vec![1, 2, 3]);
        assert_eq!(lookup.readonly_indexes, vec![4, 5, 6]);
    }

    #[test]
    fn test_try_from_ui_address_table_lookup_invalid_pubkey() {
        let ui_lookup = UiAddressTableLookup {
            account_key: "invalid_pubkey".to_string(),
            writable_indexes: vec![1, 2, 3],
            readonly_indexes: vec![4, 5, 6],
        };

        let result = MessageAddressTableLookup::try_from(&ui_lookup);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_message_address_table_lookup_to_solana_message() {
        let lookup = MessageAddressTableLookup {
            account_key: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            writable_indexes: vec![1, 2, 3],
            readonly_indexes: vec![4, 5, 6],
        };

        let solana_lookup: SolanaMessageAddressTableLookup = lookup.clone().into();
        assert_eq!(solana_lookup.account_key, lookup.account_key);
        assert_eq!(solana_lookup.writable_indexes, lookup.writable_indexes);
        assert_eq!(solana_lookup.readonly_indexes, lookup.readonly_indexes);
    }

    #[test]
    fn test_serde_serialization() {
        let lookup = MessageAddressTableLookup {
            // "11111111111111111111111111111111" decodes to 32 bytes of zero in memory
            account_key: Pubkey::from_str("11111111111111111111111111111111").unwrap(),
            writable_indexes: vec![1, 2, 3],
            readonly_indexes: vec![4, 5, 6],
        };

        let serialized = serde_json::to_string(&lookup).unwrap();
        // debug!("Serialized JSON: {}", serialized);

        // Build the 32 zeros with a local variable
        let zeros = vec![0u8; 32];
        let expected_value = serde_json::json!({
            "accountKey": zeros,                // 32 zeros for "11111111111111111111111111111111"
            "writableIndexes": [[3], 1, 2, 3],  // short-vec format
            "readonlyIndexes": [[3], 4, 5, 6]
        });

        let serialized_value: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(serialized_value, expected_value);
    }

    #[test]
    fn test_serde_deserialization() {
        // Create a 32-zero array for base58 "11111111111111111111111111111111"
        let zeros = vec![0u8; 32];
        let json_str = serde_json::json!({
            "accountKey": zeros,
            "writableIndexes": [[3], 1, 2, 3],
            "readonlyIndexes": [[3], 4, 5, 6]
        }).to_string();

        let deserialized: MessageAddressTableLookup = serde_json::from_str(&json_str).unwrap();

        // The internal bytes are all zeros, which `Pubkey::to_string()` yields as "11111111111111111111111111111111".
        assert_eq!(deserialized.account_key.to_string(), "11111111111111111111111111111111");
        assert_eq!(deserialized.writable_indexes, vec![1, 2, 3]);
        assert_eq!(deserialized.readonly_indexes, vec![4, 5, 6]);
    }
}