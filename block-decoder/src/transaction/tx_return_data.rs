use {
    crate::{
        errors::{
            conversion_error::ConversionError
        }
    },
    serde::{
        Deserialize, Serialize,
    },
    solana_sdk::pubkey::Pubkey,
    solana_transaction_status::UiTransactionReturnData,
    std::{
        str::FromStr,
    },
    base64::{Engine, prelude::BASE64_STANDARD},
};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct TransactionReturnData {
    pub program_id: Pubkey,
    pub data: Vec<u8>,
}

impl TryFrom<UiTransactionReturnData> for TransactionReturnData {
    type Error = ConversionError;

    fn try_from(ui_return_data: UiTransactionReturnData) -> Result<Self, Self::Error> {
        let program_id = Pubkey::from_str(&ui_return_data.program_id)
            .map_err(|_| ConversionError::InvalidProgramId)?;

        let data = BASE64_STANDARD.decode(&ui_return_data.data.0)
            .map_err(|_| ConversionError::InvalidData)?;

        Ok(Self { program_id, data })
    }
}


impl From<TransactionReturnData> for solana_sdk::transaction_context::TransactionReturnData {
    fn from(data: TransactionReturnData) -> Self {
        Self {
            program_id: data.program_id,
            data: data.data,
        }
    }
}