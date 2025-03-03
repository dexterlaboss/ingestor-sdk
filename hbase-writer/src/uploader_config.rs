
use {
    solana_pubkey::{
        Pubkey,
    },
    std::{
        collections::HashSet,
    },
};

pub const BLOCKS_TABLE_NAME: &str = "blocks";
pub const TX_TABLE_NAME: &str = "tx";
pub const TX_BY_ADDR_TABLE_NAME: &str = "tx-by-addr";
pub const FULL_TX_TABLE_NAME: &str = "tx_full";

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FilterTxIncludeExclude {
    pub exclude: bool,
    pub addrs: HashSet<Pubkey>,
}

#[derive(Debug, Clone)]
pub struct UploaderConfig {
    pub tx_full_filter: Option<FilterTxIncludeExclude>,
    pub tx_by_addr_filter: Option<FilterTxIncludeExclude>,
    pub disable_tx: bool,
    pub disable_tx_by_addr: bool,
    pub disable_blocks: bool,
    pub enable_full_tx: bool,
    pub blocks_table_name: String,
    pub tx_table_name: String,
    pub tx_by_addr_table_name: String,
    pub full_tx_table_name: String,
    pub use_md5_row_key_salt: bool,
    pub filter_program_accounts: bool,
    pub filter_voting_tx: bool,
    pub filter_error_tx: bool,
    pub use_blocks_compression: bool,
    pub use_tx_compression: bool,
    pub use_tx_by_addr_compression: bool,
    pub use_tx_full_compression: bool,
    pub hbase_write_to_wal: bool,
}

impl Default for UploaderConfig {
    fn default() -> Self {
        Self {
            tx_full_filter: None,
            tx_by_addr_filter: None,
            disable_tx: false,
            disable_tx_by_addr: false,
            disable_blocks: false,
            enable_full_tx: false,
            blocks_table_name: BLOCKS_TABLE_NAME.to_string(),
            tx_table_name: TX_TABLE_NAME.to_string(),
            tx_by_addr_table_name: TX_BY_ADDR_TABLE_NAME.to_string(),
            full_tx_table_name: FULL_TX_TABLE_NAME.to_string(),
            use_md5_row_key_salt: false,
            filter_program_accounts: false,
            filter_voting_tx: false,
            filter_error_tx: false,
            use_blocks_compression: true,
            use_tx_compression: true,
            use_tx_by_addr_compression: true,
            use_tx_full_compression: true,
            hbase_write_to_wal: true,
        }
    }
}