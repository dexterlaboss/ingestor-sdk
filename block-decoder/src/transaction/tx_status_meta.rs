
use {
    crate::{
        errors::{
            conversion_error::ConversionError,
        },
        instruction::{
            inner_instruction::{
                InnerInstructions,
            },
        },
        address::{
            loaded_addresses::LoadedAddresses,
        },
        transaction::{
            tx_token_balance::TransactionTokenBalance,
            tx_return_data::TransactionReturnData,
        },
    },
    solana_transaction_error::TransactionResult,
    solana_transaction_status_client_types::{
        UiTransactionStatusMeta,
        Rewards,
        option_serializer::OptionSerializer,
    },
    serde::{
        Deserialize, Serialize,
    },
};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransactionStatusMeta {
    pub status: TransactionResult<()>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub inner_instructions: Option<Vec<InnerInstructions>>,
    pub log_messages: Option<Vec<String>>,
    pub pre_token_balances: Option<Vec<TransactionTokenBalance>>,
    pub post_token_balances: Option<Vec<TransactionTokenBalance>>,
    pub rewards: Option<Rewards>,
    pub loaded_addresses: LoadedAddresses,
    pub return_data: Option<TransactionReturnData>,
    pub compute_units_consumed: Option<u64>,
}



impl TryFrom<UiTransactionStatusMeta> for TransactionStatusMeta {
    type Error = ConversionError;

    fn try_from(meta: UiTransactionStatusMeta) -> Result<Self, Self::Error> {
        let inner_instructions: Option<Vec<InnerInstructions>> = match meta.inner_instructions {
            OptionSerializer::Some(ui_inner_instructions) => {
                let inner_instructions_result: Result<Vec<_>, _> = ui_inner_instructions
                    .into_iter()
                    .map(|ui_inner_instruction| InnerInstructions::try_from(ui_inner_instruction))
                    .collect();

                match inner_instructions_result {
                    Ok(inner_instructions) => Some(inner_instructions),
                    Err(e) => return Err(e),
                }
            }
            _ => None,
        };

        let pre_token_balances: Option<Vec<TransactionTokenBalance>> = match meta.pre_token_balances {
            OptionSerializer::Some(ui_pre_token_balances) => {
                let pre_token_balances: Vec<_> = ui_pre_token_balances
                    .into_iter()
                    .map(TransactionTokenBalance::from)
                    .collect();

                Some(pre_token_balances)
            }
            _ => None,
        };

        let post_token_balances: Option<Vec<TransactionTokenBalance>> = match meta.post_token_balances {
            OptionSerializer::Some(ui_post_token_balances) => {
                let post_token_balances: Vec<_> = ui_post_token_balances
                    .into_iter()
                    .map(TransactionTokenBalance::from)
                    .collect();

                Some(post_token_balances)
            }
            _ => None,
        };

        let return_data: Option<TransactionReturnData> = match meta.return_data {
            OptionSerializer::Some(ui_return_data) => {
                let return_data = TransactionReturnData::try_from(ui_return_data)?;
                Some(return_data)
            }
            _ => None,
        };

        let loaded_addresses: LoadedAddresses = match &meta.loaded_addresses {
            OptionSerializer::Some(ui_loaded_addresses) => {
                match LoadedAddresses::try_from(ui_loaded_addresses) {
                    Ok(loaded_addresses) => loaded_addresses,
                    Err(_) => return Err(ConversionError::InvalidProgramId),
                }
            }
            _ => return Err(ConversionError::InvalidProgramId),
        };

        let compute_units_consumed: Option<u64> = match meta.compute_units_consumed {
            OptionSerializer::Some(cuc) => Some(cuc),
            _ => None,
        };

        Ok(Self {
            status: meta.status,
            fee: meta.fee,
            pre_balances: meta.pre_balances,
            post_balances: meta.post_balances,
            inner_instructions,
            log_messages: match meta.log_messages {
                OptionSerializer::Some(logs) => Some(logs),
                _ => None,
            },
            pre_token_balances,
            post_token_balances,
            rewards: match meta.rewards {
                OptionSerializer::Some(rewards) => Some(rewards),
                _ => None,
            },
            loaded_addresses,
            return_data,
            compute_units_consumed,
        })
    }
}


impl From<TransactionStatusMeta> for solana_transaction_status_client_types::TransactionStatusMeta {
    fn from(meta: TransactionStatusMeta) -> Self {
        Self {
            status: meta.status,
            fee: meta.fee,
            pre_balances: meta.pre_balances,
            post_balances: meta.post_balances,
            inner_instructions: meta.inner_instructions.map(|ii| ii.into_iter().map(Into::into).collect()),
            log_messages: meta.log_messages,
            pre_token_balances: meta.pre_token_balances.map(|tb| tb.into_iter().map(Into::into).collect()),
            post_token_balances: meta.post_token_balances.map(|tb| tb.into_iter().map(Into::into).collect()),
            rewards: meta.rewards,
            loaded_addresses: meta.loaded_addresses.into(),
            return_data: meta.return_data.map(Into::into),
            compute_units_consumed: meta.compute_units_consumed,
        }
    }
}