use {
    crate::{
        address::{
            address_table_lookup::MessageAddressTableLookup,
        },
        errors::{
            decode_error::DecodeError,
        },
        instruction::{
            CompiledInstruction,
        },
        decodable::{
            DecodableWithMeta,
        },
    },
    serde_derive::{Deserialize, Serialize},
    solana_program::short_vec,
    solana_sdk::{
        hash::Hash,
        message::{
            v0::Message as SolanaMessageV0,
            MessageHeader,
        },
        pubkey::Pubkey,
        transaction::TransactionVersion,
    },
    solana_transaction_status::{
        UiMessage,
        UiTransactionEncoding,
    },
};

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The message header, identifying signed and read-only `account_keys`.
    /// Header values only describe static `account_keys`, they do not describe
    /// any additional account keys loaded via address table lookups.
    pub header: MessageHeader,

    /// List of accounts loaded by this transaction.
    #[serde(with = "short_vec")]
    pub account_keys: Vec<Pubkey>,

    /// The blockhash of a recent block.
    pub recent_blockhash: Hash,

    /// Instructions that invoke a designated program, are executed in sequence,
    /// and committed in one atomic transaction if all succeed.
    ///
    /// # Notes
    ///
    /// Program indexes must index into the list of message `account_keys` because
    /// program id's cannot be dynamically loaded from a lookup table.
    ///
    /// Account indexes must index into the list of addresses
    /// constructed from the concatenation of three key lists:
    ///   1) message `account_keys`
    ///   2) ordered list of keys loaded from `writable` lookup table indexes
    ///   3) ordered list of keys loaded from `readable` lookup table indexes
    #[serde(with = "short_vec")]
    pub instructions: Vec<CompiledInstruction>,

    /// List of address table lookups used to load additional accounts
    /// for this transaction.
    #[serde(with = "short_vec")]
    pub address_table_lookups: Vec<MessageAddressTableLookup>,
}

impl DecodableWithMeta for Message {
    type Encoded = UiMessage;
    type Decoded = Message;

    fn decode_with_meta(
        encoded: Self::Encoded,
        decoding: UiTransactionEncoding,
        version: Option<TransactionVersion>
    ) -> Result<Self::Decoded, DecodeError> {
        match decoding {
            UiTransactionEncoding::Json => match encoded {
                UiMessage::Raw(_) => Self::json_decode(encoded, version),
                UiMessage::Parsed(_) => Err(DecodeError::UnsupportedEncoding),
            },
            _ => Err(DecodeError::UnsupportedEncoding),
        }
    }

    fn json_decode(encoded: Self::Encoded, _version: Option<TransactionVersion>) -> Result<Self::Decoded, DecodeError> {
        if let UiMessage::Raw(raw_msg) = encoded {
            let header = raw_msg.header;
            let account_keys = raw_msg.account_keys
                .iter()
                .map(|s| s.parse::<Pubkey>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DecodeError::InvalidAccountKey)?;
            let recent_blockhash = raw_msg.recent_blockhash.parse::<Hash>()
                .map_err(|_| DecodeError::InvalidBlockhash)?;
            let instructions = raw_msg.instructions
                .iter()
                .map(|i| CompiledInstruction::from(i.clone()))
                .collect::<Vec<_>>();
            let address_table_lookups = match raw_msg.address_table_lookups {
                Some(lookups) => lookups
                    .iter()
                    .map(|lookup| MessageAddressTableLookup::try_from(lookup).unwrap())
                    .collect(),
                None => vec![],
            };

            Ok(Self {
                header,
                account_keys,
                recent_blockhash,
                instructions,
                address_table_lookups,
            })
        } else {
            Err(DecodeError::UnsupportedEncoding)
        }
    }
}


impl From<Message> for SolanaMessageV0 {
    fn from(msg: Message) -> Self {
        Self {
            header: msg.header,
            account_keys: msg.account_keys,
            recent_blockhash: msg.recent_blockhash,
            instructions: msg.instructions.into_iter().map(Into::into).collect(),
            address_table_lookups: msg.address_table_lookups.into_iter().map(Into::into).collect(),
        }
    }
}