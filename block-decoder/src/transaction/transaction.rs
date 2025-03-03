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