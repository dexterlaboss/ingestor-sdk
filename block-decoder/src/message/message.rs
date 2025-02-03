use {
    crate::{
        errors::{
            decode_error::DecodeError,
        },
        instruction::{
            CompiledInstruction,
        },
        decodable::{
            Decodable,
        },
    },
    serde_derive::{Deserialize, Serialize},
    solana_program::short_vec,
    solana_sdk::{
        hash::Hash,
        message::{
            Message as SolanaMessage,
            MessageHeader,
        },
        pubkey::Pubkey,
    },
    solana_transaction_status::UiMessage,
    std::{
        str::FromStr,
    },
};

#[derive(Serialize, Deserialize, Default, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    /// The message header, identifying signed and read-only `account_keys`.
    // NOTE: Serialization-related changes must be paired with the direct read at sigverify.
    pub header: MessageHeader,

    /// All the account keys used by this transaction.
    #[serde(with = "short_vec")]
    pub account_keys: Vec<Pubkey>,

    /// The id of a recent ledger entry.
    pub recent_blockhash: Hash,

    /// Programs that will be executed in sequence and committed in one atomic transaction if all
    /// succeed.
    #[serde(with = "short_vec")]
    pub instructions: Vec<CompiledInstruction>,
}

impl Decodable for Message {
    type Encoded = UiMessage;
    type Decoded = Message;

    fn decode(encoded: &Self::Encoded) -> Result<Self::Decoded, DecodeError> {
        match encoded {
            UiMessage::Raw(raw_message) => {
                let header = raw_message.header;
                let account_keys: Result<Vec<Pubkey>, _> = raw_message
                    .account_keys
                    .iter()
                    .map(|key_str| key_str.parse())
                    .collect();
                let account_keys = account_keys?;
                let recent_blockhash = Hash::from_str(&raw_message.recent_blockhash)
                    .map_err(|err| DecodeError::ParseHashFailed(err))?;
                let instructions: Vec<CompiledInstruction> = raw_message
                    .instructions
                    .iter()
                    // .map(|ui_instruction| (*ui_instruction).into() )
                    .map(|ui_instruction| ui_instruction.clone().into() )
                    .collect();

                Ok(Message {
                    header,
                    account_keys,
                    recent_blockhash,
                    instructions,
                })
            }
            UiMessage::Parsed(_) => {
                Err(DecodeError::UnsupportedEncoding)
            }
        }
    }
}


impl From<Message> for SolanaMessage {
    fn from(msg: Message) -> Self {
        Self {
            header: msg.header,
            account_keys: msg.account_keys,
            recent_blockhash: msg.recent_blockhash,
            instructions: msg.instructions.into_iter().map(Into::into).collect(),
        }
    }
}
