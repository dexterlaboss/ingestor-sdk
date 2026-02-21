use {
    crate::{
        errors::{
            decode_error::DecodeError,
        },
        block::{
            encoded_block::{
                EncodedTransaction,
                EncodedTransactionWithStatusMeta,
            },
        },
        message::{
            message::Message,
            message_v0::Message as MessageV0,
            versioned_message::VersionedMessage,
        },
        transaction::{
            tx_status_meta::TransactionStatusMeta,
        },
        decodable::{
            Decodable,
            DecodableWithMeta,
        },
    },
    serde::{
        Deserialize, Serialize,
    },
    solana_short_vec as short_vec,
    solana_signature::Signature,
    solana_transaction::{
        versioned::TransactionVersion
    },
    solana_transaction_status_client_types::{
        UiMessage,
        UiTransactionEncoding,
        TransactionBinaryEncoding,
    },
    base64::{Engine, prelude::BASE64_STANDARD},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionedTransactionWithStatusMeta {
    pub transaction: VersionedTransaction,
    pub meta: TransactionStatusMeta,
}

impl VersionedTransactionWithStatusMeta {
    pub fn decode(
        encoded: EncodedTransactionWithStatusMeta,
        encoding: UiTransactionEncoding,
    ) -> Result<Self, DecodeError> {
        // Decoding the transaction
        let transaction = match VersionedTransaction::decode_with_meta(encoded.transaction, encoding, encoded.version /*, meta*/) {
            Ok(decoded) => decoded,
            Err(e) => return Err(e),
        };

        // Decoding the meta
        let meta = match encoded.meta {
            Some(ui_meta) => match TransactionStatusMeta::try_from(ui_meta) {
                Ok(meta) => meta,
                Err(_) => return Err(DecodeError::InvalidData),
            },
            None => return Err(DecodeError::InvalidData),
        };

        Ok(Self {
            transaction,
            meta,
        })
    }
}

#[derive(Debug, PartialEq, Default, Eq, Clone, Serialize, Deserialize)]
pub struct VersionedTransaction {
    /// List of signatures
    #[serde(with = "short_vec")]
    pub signatures: Vec<Signature>,
    /// Message to sign.
    pub message: VersionedMessage,
}


impl DecodableWithMeta for VersionedTransaction {
    type Encoded = EncodedTransaction;
    type Decoded = VersionedTransaction;

    // https://github.com/anza-xyz/agave/blob/v3.1.8/transaction-status/src/lib.rs#L632
    fn decode_with_meta(
        encoded: Self::Encoded,
        decoding: UiTransactionEncoding,
        version: Option<TransactionVersion>,
    ) -> Result<Self::Decoded, DecodeError> {
        match decoding {
            UiTransactionEncoding::Binary | UiTransactionEncoding::Base58 => {
                if let EncodedTransaction::LegacyBinary(encoded_string)
                | EncodedTransaction::Binary(
                    encoded_string,
                    TransactionBinaryEncoding::Base58,
                ) = encoded
                {
                    let decoded_bytes = bs58::decode(encoded_string)
                        .into_vec()
                        .map_err(|_| DecodeError::DeserializeFailed)?;
                    let decoded: Self::Decoded = bincode::deserialize(&decoded_bytes)
                        .map_err(|_| DecodeError::DeserializeFailed)?;
                    Ok(decoded)
                } else {
                    Err(DecodeError::UnsupportedEncoding)
                }
            }
            UiTransactionEncoding::Base64 => {
                if let EncodedTransaction::Binary(
                    encoded_string,
                    TransactionBinaryEncoding::Base64,
                ) = encoded
                {
                    let decoded_bytes = BASE64_STANDARD
                        .decode(encoded_string)
                        .map_err(|_| DecodeError::DeserializeFailed)?;
                    let decoded: Self::Decoded = bincode::deserialize(&decoded_bytes)
                        .map_err(|_| DecodeError::DeserializeFailed)?;
                    Ok(decoded)
                } else {
                    Err(DecodeError::UnsupportedEncoding)
                }
            }
            UiTransactionEncoding::Json => Self::json_decode(encoded, version),
            UiTransactionEncoding::JsonParsed => Err(DecodeError::UnsupportedEncoding),
        }
    }

    fn json_decode(encoded: Self::Encoded, version: Option<TransactionVersion>) -> Result<Self::Decoded, DecodeError> {
        if let EncodedTransaction::Json(ui_transaction) = encoded {
            let signatures = ui_transaction
                .signatures
                .iter()
                .map(|s| s.parse::<Signature>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| DecodeError::ParseSignatureFailed(err))?;

            // `EncodedTransaction::Json` encoding outputs `UiMessage::Raw`: https://github.com/anza-xyz/agave/blob/v3.1.8/transaction-status/src/lib.rs#L663
            let message = match ui_transaction.message {
                UiMessage::Raw(_) => {
                    match version {
                        Some(TransactionVersion::Number(0)) => {
                            // Handle Version 0 message decoding for raw messages
                            let v0_message = MessageV0::json_decode(ui_transaction.message, version)?;
                            VersionedMessage::V0(v0_message)
                        }
                        Some(TransactionVersion::Legacy(_)) | None => {
                            // Default to legacy message decoding for raw messages
                            let legacy_message = Message::decode(&ui_transaction.message)?;
                            VersionedMessage::Legacy(legacy_message)
                        }
                        // Add additional cases here for other versions as needed
                        _ => {
                            // Handle other versions or return an error if not supported
                            return Err(DecodeError::UnsupportedVersion);
                        }
                    }
                }
                UiMessage::Parsed(_) => {
                    return Err(DecodeError::UnsupportedEncoding);
                }
            };

            Ok(Self {
                signatures,
                message,
            })
        } else {
            Err(DecodeError::UnsupportedEncoding)
        }
    }
}

impl From<VersionedTransaction> for solana_transaction::versioned::VersionedTransaction {
    fn from(tx: VersionedTransaction) -> Self {
        Self {
            signatures: tx.signatures,
            message: tx.message.into(),
        }
    }
}

impl From<VersionedTransactionWithStatusMeta> for solana_transaction_status::VersionedTransactionWithStatusMeta {
    fn from(tx: VersionedTransactionWithStatusMeta) -> Self {
        Self {
            transaction: tx.transaction.into(),
            meta: tx.meta.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use solana_transaction_status_client_types::{
        UiTransactionEncoding, UiTransaction, UiMessage, UiRawMessage,
        UiTransactionStatusMeta,
    };
    use solana_transaction_status::TransactionBinaryEncoding;
    use serde_json::json;
    use solana_signature::SIGNATURE_BYTES;

    fn make_valid_ui_meta() -> UiTransactionStatusMeta {
        serde_json::from_value(json!({
            "status": { "Ok": null },
            "fee": 0,
            "preBalances": [100u64, 50u64],
            "postBalances": [90u64, 50u64],
            "rewards": [],
            "logMessages": [],
            "innerInstructions": [],
            "preTokenBalances": null,
            "postTokenBalances": null,
            "loadedAddresses": { "readonly": [], "writable": [] },
            "returnData": null,
            "computeUnitsConsumed": 0
        })).expect("Failed to deserialize a valid UiTransactionStatusMeta")
    }

    fn make_invalid_ui_meta() -> UiTransactionStatusMeta {
        serde_json::from_value(json!({
            "invalid": "meta"
        })).expect("Unexpectedly parsed 'invalid' meta successfully")
    }

    #[test]
    fn test_decode_versioned_transaction_with_status_meta_success() {
        // 1. Create a valid VersionedTransaction
        let valid_transaction = VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::default()),
        };

        // 2. Serialize it correctly before encoding to Base64
        let raw_bytes = bincode::serialize(&valid_transaction)
            .expect("Failed to serialize VersionedTransaction");

        let encoded_string = base64::engine::general_purpose::STANDARD.encode(&raw_bytes);

        println!("Base64 Encoded Transaction: {}", encoded_string);  // Debugging output

        // 3. Create a correctly formatted EncodedTransaction
        let encoded_tx = EncodedTransaction::Binary(
            encoded_string,
            TransactionBinaryEncoding::Base64,
        );

        // 4. Construct a valid `UiTransactionStatusMeta`
        let meta_json = json!({
            "status": { "Ok": null },
            "fee": 0,
            "preBalances": [100u64, 50u64],
            "postBalances": [90u64, 50u64],
            "rewards": [],
            "logMessages": [],
            "innerInstructions": [],
            "preTokenBalances": null,
            "postTokenBalances": null,
            "loadedAddresses": { "readonly": [], "writable": [] },
            "returnData": null,
            "computeUnitsConsumed": 0
        });

        println!("Meta JSON: {}", meta_json.to_string());  // Debugging output

        let encoded_meta = Some(serde_json::from_value(meta_json)
            .expect("Failed to create valid meta"));

        println!("Parsed Meta: {:?}", encoded_meta); // Debugging output

        let encoded = EncodedTransactionWithStatusMeta {
            transaction: encoded_tx,
            meta: encoded_meta,
            version: None,
        };

        // 5. Decode the transaction and meta
        let result = dbg!(VersionedTransactionWithStatusMeta::decode(encoded, UiTransactionEncoding::Base64));

        // Ensure decoding succeeds
        assert!(result.is_ok(), "Expected successful decoding, but got {:?}", result);
    }

    #[test]
    fn test_decode_versioned_transaction_with_status_meta_invalid_data() {
        // 1. Create a valid Base64-encoded transaction
        let valid_transaction = VersionedTransaction {
            signatures: vec![Signature::default()],  // Valid 64-byte signature
            message: VersionedMessage::Legacy(Message::default()), // Valid structure
        };

        let raw_bytes = bincode::serialize(&valid_transaction)
            .expect("Failed to serialize VersionedTransaction");

        let encoded_string = base64::engine::general_purpose::STANDARD.encode(&raw_bytes);

        let encoded_tx = EncodedTransaction::Binary(
            encoded_string,
            TransactionBinaryEncoding::Base64,
        );

        // 2. Construct an invalid `UiTransactionStatusMeta`
        let encoded_meta = Some(serde_json::from_value(json!({
            "status": { "Ok": null },
            "fee": u64::MAX,
            "preBalances": [],
            "postBalances": [],
            "rewards": [],
            "logMessages": None::<Vec<String>>,
            "innerInstructions": None::<Vec<String>>
        })).expect("Failed to create invalid meta"));

        let encoded = EncodedTransactionWithStatusMeta {
            transaction: encoded_tx,
            meta: encoded_meta,
            version: None,
        };

        // 3. Attempt to decode
        let result = VersionedTransactionWithStatusMeta::decode(encoded, UiTransactionEncoding::Base64);

        // 4. Ensure decoding fails
        match result {
            Err(DecodeError::InvalidData) => {}
            Ok(_) => panic!("Expected InvalidData error, but decoding succeeded!"),
            Err(e) => panic!("Expected InvalidData error, got {:?}", e),
        }
    }

    #[test]
    fn test_decode_versioned_transaction_with_status_meta_no_meta() {
        let valid_transaction = VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::default()),
        };

        let raw_bytes = bincode::serialize(&valid_transaction).unwrap();
        let encoded_string = base64::engine::general_purpose::STANDARD.encode(&raw_bytes);
        let encoded_tx = EncodedTransaction::Binary(encoded_string, TransactionBinaryEncoding::Base64);

        let encoded = EncodedTransactionWithStatusMeta {
            transaction: encoded_tx,
            meta: None,
            version: None,
        };

        let result = VersionedTransactionWithStatusMeta::decode(encoded, UiTransactionEncoding::Base64);
        assert!(matches!(result, Err(DecodeError::InvalidData)));
    }

    #[test]
    fn test_from_versioned_transaction() {
        // Generate a valid base58-encoded 64-byte signature
        let valid_signature_bytes = [0u8; SIGNATURE_BYTES]; // 64 bytes of zeros
        let valid_signature_str = bs58::encode(valid_signature_bytes).into_string();

        // Ensure that the signature is correctly formatted
        let signature = Signature::from_str(&valid_signature_str)
            .expect("Failed to parse valid base58 signature");

        let tx = VersionedTransaction {
            signatures: vec![signature],
            message: VersionedMessage::Legacy(Message::default()),
        };

        let converted: solana_transaction::versioned::VersionedTransaction = tx.into();

        // Ensure the number of signatures is correct
        assert_eq!(converted.signatures.len(), 1);
    }



    //
    // Encoding: Binary | Base58
    //

    #[test]
    fn test_decode_versioned_transaction_success_legacy_binary() {
        // Serialize a valid VersionedTransaction
        let raw_bytes = bincode::serialize(&VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::default()),
        }).expect("Failed to serialize VersionedTransaction");

        // Correctly encode it into base58
        let encoded_string = bs58::encode(&raw_bytes).into_string();

        // Ensure we use the correct encoding variant
        let encoded = EncodedTransaction::LegacyBinary(encoded_string);

        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::Base58,
            None,
        );

        // Ensure that decoding works as expected
        assert!(result.is_ok(), "Expected successful decoding, but got {:?}", result);
    }

    #[test]
    fn test_decode_versioned_transaction_success_base58() {
        // Serialize a valid VersionedTransaction
        let raw_bytes = bincode::serialize(&VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::default()),
        })
        .expect("Failed to serialize VersionedTransaction");

        // Correctly encode it into base58
        let encoded_string = bs58::encode(&raw_bytes).into_string();

        // Ensure we use the correct encoding variant
        let encoded = EncodedTransaction::Binary(encoded_string, TransactionBinaryEncoding::Base58);

        let result =
            VersionedTransaction::decode_with_meta(encoded, UiTransactionEncoding::Base58, None);

        // Ensure that decoding works as expected
        assert!(
            result.is_ok(),
            "Expected successful decoding, but got {:?}",
            result
        );
    }

    #[test]
    fn test_decode_versioned_transaction_invalid_base58_format() {
        let encoded = EncodedTransaction::LegacyBinary("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".to_string());
        let result = VersionedTransaction::decode_with_meta(encoded, UiTransactionEncoding::Base58, None);
        assert!(matches!(result, Err(DecodeError::DeserializeFailed)));
    }

    #[test]
    fn test_decode_versioned_transaction_invalid_legacy_binary() {
        // Provide an invalid base58 string
        let encoded = EncodedTransaction::Binary(
            "invalid-base58".to_string(),
            TransactionBinaryEncoding::Base58,
        );

        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::Base58,
            None,
        );

        // Assert that decoding fails
        assert!(result.is_err(), "Expected decoding to fail for invalid encoding type");
    }


    //
    // Encoding: Base64
    //

    #[test]
    fn test_decode_versioned_transaction_success_base64() {
        // bincode-serialize a simple VersionedTransaction
        let raw_bytes = bincode::serialize(&VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::default()),
        }).unwrap();

        let encoded_string = base64::engine::general_purpose::STANDARD.encode(&raw_bytes);

        let encoded = EncodedTransaction::Binary(
            encoded_string,
            TransactionBinaryEncoding::Base64,
        );

        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::Base64,
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_versioned_transaction_success_base64_v0() {
        // bincode-serialize a simple VersionedTransaction
        let raw_bytes = bincode::serialize(&VersionedTransaction {
            signatures: vec![Signature::default()],
            message: VersionedMessage::Legacy(Message::default()),
        })
            .unwrap();

        let encoded_string = base64::engine::general_purpose::STANDARD.encode(&raw_bytes);

        let encoded = EncodedTransaction::Binary(
            encoded_string,
            TransactionBinaryEncoding::Base64,
        );

        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::Base64,
            Some(TransactionVersion::Number(0)),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_versioned_transaction_invalid_base64() {
        // Provide an invalid base64 string
        let encoded = EncodedTransaction::Binary(
            "invalid-base64".to_string(),
            TransactionBinaryEncoding::Base64,
        );

        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::Base64,
            None,
        );
        assert!(matches!(result, Err(DecodeError::DeserializeFailed)));
    }

    #[test]
    fn test_decode_versioned_transaction_invalid_base64_v0() {
        // Provide an invalid base64 string
        let encoded = EncodedTransaction::Binary(
            "invalid-base64".to_string(),
            TransactionBinaryEncoding::Base64,
        );

        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::Base64,
            Some(TransactionVersion::Number(0)),
        );
        assert!(matches!(result, Err(DecodeError::DeserializeFailed)));
    }

    #[test]
    fn test_decode_versioned_transaction_invalid_base64_format() {
        let encoded = EncodedTransaction::Binary("aGVsbG8gd29ybGQ=".to_string(), TransactionBinaryEncoding::Base64);
        let result = VersionedTransaction::decode_with_meta(encoded, UiTransactionEncoding::Base64, None);
        assert!(matches!(result, Err(DecodeError::DeserializeFailed)));
    }


    //
    // Encoding: Json
    //

    #[test]
    fn test_decode_versioned_transaction_json_invalid_signature() {
        let ui_transaction = UiTransaction {
            signatures: vec!["invalid-signature".to_string()],
            message: UiMessage::Raw(serde_json::from_value(json!({
                "header": {
                    "numRequiredSignatures": 0,
                    "numReadonlySignedAccounts": 0,
                    "numReadonlyUnsignedAccounts": 0
                },
                "accountKeys": [],
                "instructions": [],
                "recentBlockhash": "some-blockhash"
            })).unwrap()),
        };

        let json_encoded = EncodedTransaction::Json(ui_transaction);
        let result = VersionedTransaction::decode_with_meta(json_encoded, UiTransactionEncoding::Json, None);
        assert!(matches!(result, Err(DecodeError::ParseSignatureFailed(_))));
    }

    #[test]
    fn test_decode_versioned_transaction_json_invalid_version() {
        let ui_transaction = UiTransaction {
            signatures: vec![Signature::default().to_string()],
            message: UiMessage::Raw(serde_json::from_value(json!({
                "header": {
                    "numRequiredSignatures": 0,
                    "numReadonlySignedAccounts": 0,
                    "numReadonlyUnsignedAccounts": 0
                },
                "accountKeys": [],
                "instructions": [],
                "recentBlockhash": "some-blockhash"
            })).unwrap()),
        };

        let json_encoded = EncodedTransaction::Json(ui_transaction);
        let result = VersionedTransaction::decode_with_meta(
            json_encoded,
            UiTransactionEncoding::Json,
            Some(TransactionVersion::Number(99))
        );
        assert!(matches!(result, Err(DecodeError::UnsupportedVersion)));
    }

    #[test]
    fn test_decode_versioned_transaction_invalid_json_encoding() {
        let ui_transaction = UiTransaction {
            signatures: vec![Signature::default().to_string()],
            message: UiMessage::Parsed(serde_json::from_value(json!({
                "accountKeys": [],
                "recentBlockhash": "some-blockhash",
                "instructions": []
            })).unwrap()),
        };

        let json_encoded = EncodedTransaction::Json(ui_transaction);
        let result = VersionedTransaction::decode_with_meta(
            json_encoded,
            UiTransactionEncoding::JsonParsed,
            None
        );
        assert!(matches!(result, Err(DecodeError::UnsupportedEncoding)));
    }

    #[test]
    fn test_decode_versioned_transaction_json() {
        let ui_transaction = UiTransaction {
            signatures: vec!["signature".to_string()],
            message: UiMessage::Raw(
                serde_json::from_value(json!({
                    "header": {
                        "numRequiredSignatures": 0,
                        "numReadonlySignedAccounts": 0,
                        "numReadonlyUnsignedAccounts": 0
                    },
                    "accountKeys": [],
                    "instructions": [],
                    "recentBlockhash": "some-blockhash"
                })).unwrap()
            ),
        };

        let json_encoded = EncodedTransaction::Json(ui_transaction);

        let result = VersionedTransaction::decode_with_meta(
            json_encoded,
            UiTransactionEncoding::Json,
            Some(TransactionVersion::Number(0)),
        );

        assert!(result.is_err());
    }


    //
    // Encoding: JsonParsed
    //

    #[test]
    fn test_decode_versioned_transaction_unsupported_encoding() {
        // Construct a valid JSON-encoded transaction
        let ui_transaction = UiTransaction {
            signatures: vec!["signature".to_string()],
            message: UiMessage::Parsed(serde_json::from_value(json!({
                "accountKeys": [],
                "recentBlockhash": "some-blockhash",
                "instructions": []
            })).expect("Failed to construct valid UiMessage::Parsed")),
        };

        let json_encoded = EncodedTransaction::Json(ui_transaction);

        let result = VersionedTransaction::decode_with_meta(
            json_encoded,
            UiTransactionEncoding::JsonParsed, // Using JsonParsed to trigger unsupported error
            None,
        );

        // Ensure decoding fails with UnsupportedEncoding
        assert!(result.is_err(), "Expected decoding to fail with UnsupportedEncoding");
    }

    #[test]
    fn test_decode_versioned_transaction_unsupported_encoding_variant() {
        let encoded = EncodedTransaction::Binary("invalid".to_string(), TransactionBinaryEncoding::Base64);
        let result = VersionedTransaction::decode_with_meta(
            encoded,
            UiTransactionEncoding::JsonParsed,
            None
        );
        assert!(matches!(result, Err(DecodeError::UnsupportedEncoding)));
    }
}