
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