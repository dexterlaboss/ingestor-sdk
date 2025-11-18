use {
    crate::{
        errors::{
            decode_error::DecodeError,
        },
        block::{
            encoded_block::EncodedTransaction,
        },
        message::{
            message::Message,
        },
        decodable::{
            Decodable,
        },
    },
    serde_derive::{Deserialize, Serialize},
    solana_short_vec as short_vec,
    solana_signature::{
        ParseSignatureError,
        Signature,
    },
    solana_transaction_status_client_types::{
        TransactionBinaryEncoding,
        // UiTransaction,
    },
    std::{
        str::FromStr,
    },
    base64::{Engine, prelude::BASE64_STANDARD},
};

#[derive(Debug, PartialEq, Default, Eq, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// A set of signatures of a serialized [`Message`], signed by the first
    /// keys of the `Message`'s [`account_keys`], where the number of signatures
    /// is equal to [`num_required_signatures`] of the `Message`'s
    /// [`MessageHeader`].
    ///
    /// [`account_keys`]: Message::account_keys
    /// [`MessageHeader`]: crate::message::MessageHeader
    /// [`num_required_signatures`]: crate::message::MessageHeader::num_required_signatures
    // NOTE: Serialization-related changes must be paired with the direct read at sigverify.
    #[serde(with = "short_vec")]
    pub signatures: Vec<Signature>,

    /// The message to sign.
    pub message: Message,
}

impl Decodable for Transaction {
    type Encoded = EncodedTransaction;
    type Decoded = Transaction;

    fn decode(encoded: &Self::Encoded) -> Result<Self::Decoded, DecodeError> {
        match encoded {
            EncodedTransaction::LegacyBinary(s) | EncodedTransaction::Binary(s, TransactionBinaryEncoding::Base58) => {
                let data = bs58::decode(s)
                    .into_vec()
                    .map_err(|_| DecodeError::DeserializeFailed)?;
                let transaction: Transaction = bincode::deserialize(&data)
                    .map_err(|_| DecodeError::DeserializeFailed)?;
                Ok(transaction)
            }
            EncodedTransaction::Binary(s, TransactionBinaryEncoding::Base64) => {
                let data = BASE64_STANDARD.decode(s)
                    .map_err(|_| DecodeError::DeserializeFailed)?;
                let transaction: Transaction = bincode::deserialize(&data)
                    .map_err(|_| DecodeError::DeserializeFailed)?;
                Ok(transaction)
            }
            EncodedTransaction::Json(ui_transaction) => {
                let message = Message::decode(&ui_transaction.message)?;
                let signatures: Result<Vec<Signature>, ParseSignatureError> = ui_transaction.signatures.iter()
                    .map(|s| Signature::from_str(s))
                    .collect();
                let signatures = match signatures {
                    Ok(signatures) => signatures,
                    Err(error) => return Err(DecodeError::ParseSignatureFailed(error)),
                };
                Ok(Transaction {
                    signatures,
                    message,
                })
            }
            EncodedTransaction::Accounts(_) => {
                Err(DecodeError::UnsupportedEncoding)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode;
    use bs58;
    use base64::{Engine, prelude::BASE64_STANDARD};
    use solana_transaction_status::{
        TransactionBinaryEncoding, UiTransaction, UiMessage, UiRawMessage, UiAccountsList, UiCompiledInstruction, UiAddressTableLookup,
    };
    // use solana_sdk::message::MessageHeader;
    use solana_message::{
        MessageHeader,
    };
    use crate::errors::decode_error::DecodeError;
    use crate::block::encoded_block::EncodedTransaction;
    use crate::message::message::Message;
    use std::str::FromStr;
    use solana_signature::Signature;

    fn create_sample_message() -> Message {
        Message::default()
    }

    fn create_sample_ui_message() -> UiMessage {
        UiMessage::Raw(UiRawMessage {
            header: MessageHeader {
                num_required_signatures: 1,
                num_readonly_signed_accounts: 0,
                num_readonly_unsigned_accounts: 0,
            },
            account_keys: vec!["11111111111111111111111111111111".to_string()],
            recent_blockhash: "11111111111111111111111111111111".to_string(),
            instructions: vec![],
            address_table_lookups: None,
        })
    }

    fn create_sample_transaction() -> Transaction {
        Transaction {
            signatures: vec![Signature::default()],
            message: create_sample_message(),
        }
    }

    #[test]
    fn test_decode_legacy_binary_success() {
        let transaction = create_sample_transaction();
        let serialized = bincode::serialize(&transaction).expect("Failed to serialize transaction");
        let encoded = bs58::encode(&serialized).into_string();
        let encoded_transaction = EncodedTransaction::LegacyBinary(encoded);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), transaction);
    }

    #[test]
    fn test_decode_base58_binary_success() {
        let transaction = create_sample_transaction();
        let serialized = bincode::serialize(&transaction).expect("Failed to serialize transaction");
        let encoded = bs58::encode(&serialized).into_string();
        let encoded_transaction = EncodedTransaction::Binary(encoded, TransactionBinaryEncoding::Base58);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), transaction);
    }

    #[test]
    fn test_decode_base64_binary_success() {
        let transaction = create_sample_transaction();
        let serialized = bincode::serialize(&transaction).expect("Failed to serialize transaction");
        let encoded = BASE64_STANDARD.encode(&serialized);
        let encoded_transaction = EncodedTransaction::Binary(encoded, TransactionBinaryEncoding::Base64);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), transaction);
    }

    #[test]
    fn test_decode_invalid_base58_binary_fails() {
        let encoded_transaction = EncodedTransaction::Binary("invalidbase58".to_string(), TransactionBinaryEncoding::Base58);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_err());
        assert_eq!(decoded.unwrap_err(), DecodeError::DeserializeFailed);
    }

    #[test]
    fn test_decode_invalid_base64_binary_fails() {
        let encoded_transaction = EncodedTransaction::Binary("invalidbase64==".to_string(), TransactionBinaryEncoding::Base64);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_err());
        assert_eq!(decoded.unwrap_err(), DecodeError::DeserializeFailed);
    }

    #[test]
    fn test_decode_json_success() {
        let message = create_sample_ui_message();
        let ui_transaction = UiTransaction {
            signatures: vec![Signature::default().to_string()],
            message: message.clone(),
        };
        let encoded_transaction = EncodedTransaction::Json(ui_transaction);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_ok(), "Transaction decoding should succeed, got {:?}", decoded);
        if let Ok(result) = decoded {
            assert_eq!(result.signatures.len(), 1);
        }
    }

    #[test]
    fn test_decode_json_invalid_signature_fails() {
        let message = create_sample_ui_message();
        let ui_transaction = UiTransaction {
            signatures: vec!["invalid_signature".to_string()],
            message,
        };
        let encoded_transaction = EncodedTransaction::Json(ui_transaction);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(matches!(decoded, Err(DecodeError::ParseSignatureFailed(_)) | Err(DecodeError::UnsupportedEncoding)),
                "Expected ParseSignatureFailed or UnsupportedEncoding error, got {:?}", decoded);
    }

    #[test]
    fn test_decode_accounts_encoding_fails() {
        let encoded_transaction = EncodedTransaction::Accounts(UiAccountsList { account_keys: vec![], signatures: vec![] });
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_err());
        assert_eq!(decoded.unwrap_err(), DecodeError::UnsupportedEncoding);
    }

    #[test]
    fn test_decode_empty_base58_fails() {
        let encoded_transaction = EncodedTransaction::Binary("".to_string(), TransactionBinaryEncoding::Base58);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_err());
    }

    #[test]
    fn test_decode_empty_base64_fails() {
        let encoded_transaction = EncodedTransaction::Binary("".to_string(), TransactionBinaryEncoding::Base64);
        let decoded = Transaction::decode(&encoded_transaction);
        assert!(decoded.is_err());
    }
}
