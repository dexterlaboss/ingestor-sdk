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
    solana_short_vec as short_vec,
    solana_hash::{
        Hash,
    },
    solana_message::{
        Message as SolanaMessage,
        MessageHeader,
    },
    solana_pubkey::Pubkey,
    solana_transaction_status_client_types::{
        UiMessage,
    },
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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_transaction_status_client_types::{UiCompiledInstruction, UiMessage, UiRawMessage, UiParsedMessage};
    use solana_hash::Hash;
    use solana_pubkey::Pubkey;
    use std::str::FromStr;
    use serde_json;
    use bs58;

    fn sample_ui_message() -> UiMessage {
        UiMessage::Raw(UiRawMessage {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 1,
            },
            account_keys: vec![Pubkey::new_unique().to_string()],
            recent_blockhash: Hash::new_unique().to_string(),
            instructions: vec![UiCompiledInstruction {
                program_id_index: 0,
                accounts: vec![0],
                data: bs58::encode(b"test_data").into_string(),
                stack_height: None,
            }],
            address_table_lookups: None,
        })
    }

    #[test]
    fn test_message_decoding_valid() {
        let ui_message = sample_ui_message();
        let decoded_message = Message::decode(&ui_message);
        assert!(decoded_message.is_ok(), "Expected decoding to succeed");
    }

    #[test]
    fn test_message_decoding_invalid_pubkey() {
        let mut ui_message = sample_ui_message();
        if let UiMessage::Raw(ref mut raw_message) = ui_message {
            raw_message.account_keys[0] = "invalid_pubkey".to_string();
        }

        let decoded_message = Message::decode(&ui_message);
        assert!(decoded_message.is_err(), "Expected decoding to fail due to invalid pubkey");
    }

    #[test]
    fn test_message_decoding_invalid_hash() {
        let mut ui_message = sample_ui_message();
        if let UiMessage::Raw(ref mut raw_message) = ui_message {
            raw_message.recent_blockhash = "invalid_hash".to_string();
        }

        let decoded_message = Message::decode(&ui_message);
        assert!(decoded_message.is_err(), "Expected decoding to fail due to invalid hash");
    }

    // #[test]
    // fn test_message_deserialization() {
    //     let message_json = r#"{
    //         \"header\": { \"num_required_signatures\": 1, \"num_readonly_signed_accounts\": 0, \"num_readonly_unsigned_accounts\": 1 },
    //         \"account_keys\": [\"11111111111111111111111111111111\"],
    //         \"recent_blockhash\": \"11111111111111111111111111111111\",
    //         \"instructions\": [
    //             { \"program_id_index\": 0, \"accounts\": [0], \"data\": \"3q2+7w==\" }
    //         ]
    //     }"#;
    //
    //     let message: Result<Message, serde_json::Error> = serde_json::from_str(message_json);
    //     assert!(message.is_ok(), "Message deserialization failed: {:?}", message.err());
    //     let message = message.unwrap();
    //
    //     assert_eq!(message.header.num_required_signatures, 1);
    //     assert_eq!(message.header.num_readonly_signed_accounts, 0);
    //     assert_eq!(message.header.num_readonly_unsigned_accounts, 1);
    //     assert_eq!(message.account_keys.len(), 1);
    //     assert_eq!(message.instructions.len(), 1);
    // }

    #[test]
    fn test_message_decoding_unsupported_encoding() {
        let ui_message = UiMessage::Parsed(UiParsedMessage {
            account_keys: vec![],
            recent_blockhash: "".to_string(),
            instructions: vec![],
            address_table_lookups: None,
        });
        let decoded_message = Message::decode(&ui_message);
        assert!(matches!(decoded_message, Err(DecodeError::UnsupportedEncoding)));
    }

    #[test]
    fn test_message_conversion_to_solana_message() {
        let ui_message = sample_ui_message();
        let decoded_message = Message::decode(&ui_message).expect("Decoding failed");
        let solana_message: SolanaMessage = decoded_message.clone().into();

        assert_eq!(decoded_message.header, solana_message.header);
        assert_eq!(decoded_message.account_keys, solana_message.account_keys);
        assert_eq!(decoded_message.recent_blockhash, solana_message.recent_blockhash);
        assert_eq!(decoded_message.instructions.len(), solana_message.instructions.len());
    }

    #[test]
    fn test_message_with_multiple_account_keys() {
        let mut ui_message = sample_ui_message();
        if let UiMessage::Raw(ref mut raw_message) = ui_message {
            raw_message.account_keys.push(Pubkey::new_unique().to_string());
        }
        let decoded_message = Message::decode(&ui_message);
        assert!(decoded_message.is_ok(), "Expected decoding to succeed with multiple account keys");
        assert_eq!(decoded_message.unwrap().account_keys.len(), 2);
    }
}
