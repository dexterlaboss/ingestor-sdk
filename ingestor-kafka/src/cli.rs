use {
    clap::{
        App,
        Arg,
    },
    solana_clap_utils::{
        input_validators::{
            is_pubkey,
            is_parsable,
            is_within_range,
        },
    },
};

const EXCLUDE_TX_FULL_ADDR: &str = "filter-tx-full-exclude-addr";
const INCLUDE_TX_FULL_ADDR: &str = "filter-tx-full-include-addr";

const EXCLUDE_TX_BY_ADDR_ADDR: &str = "filter-tx-by-addr-exclude-addr";
const INCLUDE_TX_BY_ADDR_ADDR: &str = "filter-tx-by-addr-include-addr";

pub fn block_uploader_app<'a>(version: &'a str, _default_args: &'a DefaultBlockUploaderArgs) -> App<'a, 'a> {
    App::new("solana-block-uploader-service")
        .about("Solana Block Uploader Service")
        .version(version)
        .arg(
            Arg::with_name("disable_tx")
                .long("disable-tx")
                .takes_value(false)
                .help("Enable historical transaction info over JSON RPC, \
                       including the 'getConfirmedBlock' API."),
        )
        .arg(
            Arg::with_name("disable_tx_by_addr")
                .long("disable-tx-by-addr")
                .takes_value(false)
                .help("Enable historical transaction info over JSON RPC, \
                       including the 'getConfirmedBlock' API."),
        )
        .arg(
            Arg::with_name("disable_blocks")
                .long("disable-blocks")
                .takes_value(false)
                .help("Enable historical transaction info over JSON RPC, \
                       including the 'getConfirmedBlock' API."),
        )
        .arg(
            Arg::with_name("enable_full_tx")
                .long("enable-full-tx")
                .takes_value(false)
                .help("Enable historical transaction info over JSON RPC, \
                       including the 'getConfirmedBlock' API."),
        )
        .arg(
            Arg::with_name("use_md5_row_key_salt")
                .long("use-md5-row-key-salt")
                .takes_value(false)
                .help("Add md5 salt to hbase row keys."),
        )
        .arg(
            Arg::with_name("filter_tx_by_addr_programs")
                .long("filter-tx-by-addr-programs")
                .takes_value(false)
                .help("Skip program accounts from tx-by-addr index."),
        )
        .arg(
            Arg::with_name("filter_voting_tx")
                .long("filter-voting-tx")
                .takes_value(false)
                .help("Do not store voting transactions in tx-by-addr and tx_full."),
        )
        .arg(
            Arg::with_name("filter_error_tx")
                .long("filter-error-tx")
                .takes_value(false)
                .help("Do not store failed transactions in tx-by-addr and tx_full."),
        )
        .arg(
            Arg::with_name("disable_blocks_compression")
                .long("disable-blocks-compression")
                .takes_value(false)
                .help("Disables blocks table compression."),
        )
        .arg(
            Arg::with_name("disable_tx_compression")
                .long("disable-tx-compression")
                .takes_value(false)
                .help("Disables tx table compression."),
        )
        .arg(
            Arg::with_name("disable_tx_by_addr_compression")
                .long("disable-tx-by-addr-compression")
                .takes_value(false)
                .help("Disables tx-by-addr table compression."),
        )
        .arg(
            Arg::with_name("disable_tx_full_compression")
                .long("disable-tx-full-compression")
                .takes_value(false)
                .help("Disables tx-full table compression."),
        )
        .arg(
            Arg::with_name("filter_tx_full_include_addr")
                .long(INCLUDE_TX_FULL_ADDR)
                .takes_value(true)
                .validator(is_pubkey)
                .multiple(true)
                .value_name("KEY")
                .help("Store only transactions with this account key in tx-full."),
        )
        .arg(
            Arg::with_name("filter_tx_full_exclude_addr")
                .long(EXCLUDE_TX_FULL_ADDR)
                .takes_value(true)
                .validator(is_pubkey)
                .conflicts_with("filter_tx_full_include_addr")
                .multiple(true)
                .value_name("KEY")
                .help("Store all transactions in tx-full except the ones with this account key. Overrides filter_tx_full_include_addr."),
        )
        .arg(
            Arg::with_name("filter_tx_by_addr_include_addr")
                .long(INCLUDE_TX_BY_ADDR_ADDR)
                .takes_value(true)
                .validator(is_pubkey)
                .multiple(true)
                .value_name("KEY")
                .help("Store only transactions with this account key in tx-by-addr."),
        )
        .arg(
            Arg::with_name("filter_tx_by_addr_exclude_addr")
                .long(EXCLUDE_TX_BY_ADDR_ADDR)
                .takes_value(true)
                .validator(is_pubkey)
                .conflicts_with("filter_tx_by_addr_include_addr")
                .multiple(true)
                .value_name("KEY")
                .help("Store all transactions in tx-by-addr except the ones with this account key. Overrides filter_tx_by_addr_include_addr."),
        )
        .arg(
            Arg::with_name("enable_full_tx_cache")
                .long("enable-full-tx-cache")
                .takes_value(false)
                .help("Enable block transaction cache."),
        )
        .arg(
            Arg::with_name("cache_timeout")
                .long("cache-timeout")
                .value_name("SECONDS")
                .validator(is_parsable::<u64>)
                .takes_value(true)
                // .default_value(&default_args.cache_timeout)
                .help("Cache connection timeout"),
        )
        .arg(
            Arg::with_name("tx_cache_expiration")
                .long("tx-cache-expiration")
                .value_name("DAYS")
                .validator(|v| is_within_range::<usize, _>(v, 0..=30))
                .takes_value(true)
                // .default_value(&default_args.tx_cache_expiration)
                .help("Number of days before tx cache records expire"),
        )
        .arg(
            Arg::with_name("cache_address")
                .long("cache-address")
                .value_name("ADDRESS")
                .takes_value(true)
                // .default_value(&default_args.cache_address)
                .help("Address of the cache server"),
        )
        .arg(
            Arg::with_name("hbase_skip_wal")
                .long("hbase-skip-wal")
                .takes_value(false)
                .help("If HBase should skip WAL when writing new data."),
        )
}

pub struct DefaultBlockUploaderArgs {
    pub disable_tx: bool,
    pub disable_tx_by_addr: bool,
    pub disable_blocks: bool,
    pub enable_full_tx: bool,
    pub use_md5_row_key_salt: bool,
    pub disable_blocks_compression: bool,
    pub disable_tx_compression: bool,
    pub disable_tx_by_addr_compression: bool,
    pub disable_tx_full_compression: bool,
    pub enable_full_tx_cache: bool,
}

impl DefaultBlockUploaderArgs {
    pub fn new() -> Self {
        DefaultBlockUploaderArgs {
            disable_tx: false,
            disable_tx_by_addr: false,
            disable_blocks: false,
            enable_full_tx: false,
            use_md5_row_key_salt: false,
            disable_blocks_compression: false,
            disable_tx_compression: false,
            disable_tx_by_addr_compression: false,
            disable_tx_full_compression: false,
            enable_full_tx_cache: false,
        }
    }
}

impl Default for DefaultBlockUploaderArgs {
    fn default() -> Self {
        Self::new()
    }
}