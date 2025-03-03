use {
    // solana_account_decoder::parse_token::UiTokenAmount,
    solana_account_decoder_client_types::{
        token::UiTokenAmount,
    },
    // solana_transaction_status::{
    //     UiTransactionTokenBalance,
    //     option_serializer::OptionSerializer,
    // },
    solana_transaction_status_client_types::{
        UiTransactionTokenBalance,
        option_serializer::OptionSerializer,
    },
    serde::{
        Deserialize, Serialize,
    },
};


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransactionTokenBalance {
    pub account_index: u8,
    pub mint: String,
    pub ui_token_amount: UiTokenAmount,
    pub owner: String,
    pub program_id: String,
}

impl From<UiTransactionTokenBalance> for TransactionTokenBalance {
    fn from(token_balance: UiTransactionTokenBalance) -> Self {
        Self {
            account_index: token_balance.account_index,
            mint: token_balance.mint,
            ui_token_amount: token_balance.ui_token_amount,
            owner: match token_balance.owner {
                OptionSerializer::Some(owner) => owner,
                _ => String::new(),
            },
            program_id: match token_balance.program_id {
                OptionSerializer::Some(program_id) => program_id,
                _ => String::new(),
            },
        }
    }
}


impl From<TransactionTokenBalance> for solana_transaction_status_client_types::TransactionTokenBalance {
    fn from(balance: TransactionTokenBalance) -> Self {
        Self {
            account_index: balance.account_index,
            mint: balance.mint,
            ui_token_amount: balance.ui_token_amount,
            owner: balance.owner,
            program_id: balance.program_id,
        }
    }
}