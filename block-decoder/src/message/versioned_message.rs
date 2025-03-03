use {
    crate::{
        message::{
            message::Message,
            message_v0::Message as MessageV0,
        },
        instruction::{
            CompiledInstruction,
        },
    },
    solana_short_vec as short_vec,
    solana_message::{
        MessageHeader,
    },
    solana_pubkey::{
        Pubkey,
    },
    solana_hash::{
        Hash,
    },
    serde::{
        de::{self, Deserializer, SeqAccess, Visitor},
        ser::{SerializeTuple, Serializer},
        Deserialize, Serialize,
    },
    std::{fmt},
};

pub const MESSAGE_VERSION_PREFIX: u8 = 0x80;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum VersionedMessage {
    Legacy(Message),
    V0(MessageV0),
}

impl Default for VersionedMessage {
    fn default() -> Self {
        Self::Legacy(Message::default())
    }
}

impl Serialize for VersionedMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Legacy(message) => {
                let mut seq = serializer.serialize_tuple(1)?;
                seq.serialize_element(message)?;
                seq.end()
            }
            Self::V0(message) => {
                let mut seq = serializer.serialize_tuple(2)?;
                seq.serialize_element(&MESSAGE_VERSION_PREFIX)?;
                seq.serialize_element(message)?;
                seq.end()
            }
        }
    }
}

enum MessagePrefix {
    Legacy(u8),
    Versioned(u8),
}

impl<'de> Deserialize<'de> for MessagePrefix {
    fn deserialize<D>(deserializer: D) -> Result<MessagePrefix, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PrefixVisitor;

        impl<'de> Visitor<'de> for PrefixVisitor {
            type Value = MessagePrefix;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("message prefix byte")
            }

            fn visit_u8<E>(self, byte: u8) -> Result<MessagePrefix, E> {
                if byte & MESSAGE_VERSION_PREFIX != 0 {
                    Ok(MessagePrefix::Versioned(byte & !MESSAGE_VERSION_PREFIX))
                } else {
                    Ok(MessagePrefix::Legacy(byte))
                }
            }
        }

        deserializer.deserialize_u8(PrefixVisitor)
    }
}

impl<'de> Deserialize<'de> for VersionedMessage {
    fn deserialize<D>(deserializer: D) -> Result<VersionedMessage, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MessageVisitor;

        impl<'de> Visitor<'de> for MessageVisitor {
            type Value = VersionedMessage;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("message bytes")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<VersionedMessage, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let prefix: MessagePrefix = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                match prefix {
                    MessagePrefix::Legacy(num_required_signatures) => {
                        // The remaining fields of the legacy Message struct after the first byte.
                        #[derive(Serialize, Deserialize)]
                        struct RemainingLegacyMessage {
                            pub num_readonly_signed_accounts: u8,
                            pub num_readonly_unsigned_accounts: u8,
                            #[serde(with = "short_vec")]
                            pub account_keys: Vec<Pubkey>,
                            pub recent_blockhash: Hash,
                            #[serde(with = "short_vec")]
                            pub instructions: Vec<CompiledInstruction>,
                        }

                        let message: RemainingLegacyMessage =
                            seq.next_element()?.ok_or_else(|| {
                                // will never happen since tuple length is always 2
                                de::Error::invalid_length(1, &self)
                            })?;

                        Ok(VersionedMessage::Legacy(Message {
                            header: MessageHeader {
                                num_required_signatures,
                                num_readonly_signed_accounts: message.num_readonly_signed_accounts,
                                num_readonly_unsigned_accounts: message
                                    .num_readonly_unsigned_accounts,
                            },
                            account_keys: message.account_keys,
                            recent_blockhash: message.recent_blockhash,
                            instructions: message.instructions,
                        }))
                    }
                    MessagePrefix::Versioned(version) => {
                        match version {
                            0 => {
                                Ok(VersionedMessage::V0(seq.next_element()?.ok_or_else(
                                    || {
                                        // will never happen since tuple length is always 2
                                        de::Error::invalid_length(1, &self)
                                    },
                                )?))
                            }
                            127 => {
                                // 0xff is used as the first byte of the off-chain messages
                                // which corresponds to version 127 of the versioned messages.
                                // This explicit check is added to prevent the usage of version 127
                                // in the runtime as a valid transaction.
                                Err(de::Error::custom("off-chain messages are not accepted"))
                            }
                            _ => Err(de::Error::invalid_value(
                                de::Unexpected::Unsigned(version as u64),
                                &"a valid transaction message version",
                            )),
                        }
                    }
                }
            }
        }

        deserializer.deserialize_tuple(2, MessageVisitor)
    }
}

impl From<VersionedMessage> for solana_message::VersionedMessage {
    fn from(msg: VersionedMessage) -> Self {
        match msg {
            VersionedMessage::Legacy(m) => solana_message::VersionedMessage::Legacy(m.into()),
            VersionedMessage::V0(m) => solana_message::VersionedMessage::V0(m.into()),
        }
    }
}