
use {
    solana_block_decoder::{
        block::{
            encoded_block::EncodedConfirmedBlock,
        },
        convert_block,
    },
    ingestor_kafka::{
        config::Config,
        cli::{DefaultBlockUploaderArgs, block_uploader_app},
    },
    solana_transaction_status::{
        BlockEncodingOptions,
    },
    solana_transaction_status_client_types::{
        UiTransactionEncoding,
        TransactionDetails,
    },
    solana_hbase_writer::{
        ledger_storage::LedgerStorage,
        storage_config::LedgerStorageConfig,
        uploader_config::FilterTxIncludeExclude,
        uploader_config::UploaderConfig,
        cache_config::LedgerCacheConfig,
    },
    solana_storage_writer::{
        LedgerStorageAdapter,
    },
    std::{
        sync::Arc,
        collections::{HashSet},
        io,
        io::{Read},
    },
    solana_pubkey::{
        Pubkey,
    },
    clap::{value_t_or_exit, values_t, ArgMatches},
    serde::{
        Deserialize,
        Serialize,
    },
    serde_json,
};


// pub async fn output_block(
//     block: VersionedConfirmedBlock
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let data: Vec<u8> = encode_block::<generated::ConfirmedBlock>(block.into()).await?;
//
//     let stdout = io::stdout();
//     let mut handle = stdout.lock();
//     handle.write_all(&data)?;
//
//     Ok(())
// }

fn process_uploader_arguments(matches: &ArgMatches) -> UploaderConfig {
    let disable_tx = matches.is_present("disable_tx");
    let disable_tx_by_addr = matches.is_present("disable_tx_by_addr");
    let disable_blocks = matches.is_present("disable_blocks");
    let enable_full_tx = matches.is_present("enable_full_tx");
    let use_blocks_compression = !matches.is_present("disable_blocks_compression");
    let use_tx_compression = !matches.is_present("disable_tx_compression");
    let use_tx_by_addr_compression = !matches.is_present("disable_tx_by_addr_compression");
    let use_tx_full_compression = !matches.is_present("disable_tx_full_compression");
    let hbase_write_to_wal = !matches.is_present("hbase_skip_wal");

    let filter_tx_full_include_addrs: HashSet<Pubkey> =
        values_t!(matches, "filter_tx_full_include_addr", Pubkey)
            .unwrap_or_default()
            .iter()
            .cloned()
            .collect();

    let filter_tx_full_exclude_addrs: HashSet<Pubkey> =
        values_t!(matches, "filter_tx_full_exclude_addr", Pubkey)
            .unwrap_or_default()
            .iter()
            .cloned()
            .collect();

    let filter_tx_by_addr_include_addrs: HashSet<Pubkey> =
        values_t!(matches, "filter_tx_by_addr_include_addr", Pubkey)
            .unwrap_or_default()
            .iter()
            .cloned()
            .collect();

    let filter_tx_by_addr_exclude_addrs: HashSet<Pubkey> =
        values_t!(matches, "filter_tx_by_addr_exclude_addr", Pubkey)
            .unwrap_or_default()
            .iter()
            .cloned()
            .collect();

    let tx_full_filter = create_filter(
        filter_tx_full_exclude_addrs,
        filter_tx_full_include_addrs
    );
    let tx_by_addr_filter = create_filter(
        filter_tx_by_addr_exclude_addrs,
        filter_tx_by_addr_include_addrs
    );

    UploaderConfig {
        tx_full_filter,
        tx_by_addr_filter,
        disable_tx,
        disable_tx_by_addr,
        disable_blocks,
        enable_full_tx,
        blocks_table_name: "blocks_test".to_string(),
        tx_table_name: "tx_test".to_string(),
        tx_by_addr_table_name: "tx-by-addr_test".to_string(),
        full_tx_table_name: "tx_full_test".to_string(),
        use_md5_row_key_salt: false,
        filter_program_accounts: false,
        filter_voting_tx: false,
        filter_error_tx: false,
        use_blocks_compression,
        use_tx_compression,
        use_tx_by_addr_compression,
        use_tx_full_compression,
        hbase_write_to_wal,
    }
}

fn process_cache_arguments(matches: &ArgMatches) -> LedgerCacheConfig {
    let enable_full_tx_cache = matches.is_present("enable_full_tx_cache");

    let address = if matches.is_present("cache_address") {
        value_t_or_exit!(matches, "cache_address", String)
    } else {
        String::new()
    };

    let timeout = if matches.is_present("cache_timeout") {
        Some(std::time::Duration::from_secs(
            value_t_or_exit!(matches, "cache_timeout", u64),
        ))
    } else {
        None
    };

    let tx_cache_expiration = if matches.is_present("tx_cache_expiration") {
        Some(std::time::Duration::from_secs(
            value_t_or_exit!(matches, "tx_cache_expiration", u64) * 24 * 60 * 60,
        ))
    } else {
        None
    };

    LedgerCacheConfig {
        enable_full_tx_cache,
        address,
        timeout,
        tx_cache_expiration,
        ..Default::default()
    }
}

fn create_filter(
    filter_tx_exclude_addrs: HashSet<Pubkey>,
    filter_tx_include_addrs: HashSet<Pubkey>,
) -> Option<FilterTxIncludeExclude> {
    let exclude_tx_addrs = !filter_tx_exclude_addrs.is_empty();
    let include_tx_addrs = !filter_tx_include_addrs.is_empty();

    if exclude_tx_addrs || include_tx_addrs {
        let filter_tx_addrs = FilterTxIncludeExclude {
            exclude: exclude_tx_addrs,
            addrs: if exclude_tx_addrs {
                filter_tx_exclude_addrs
            } else {
                filter_tx_include_addrs
            },
        };
        Some(filter_tx_addrs)
    } else {
        None
    }
}

#[derive(Serialize, Deserialize)]
struct NumberContainer {
    number: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_args = DefaultBlockUploaderArgs::new();
    let solana_version = solana_version::version!();
    let cli_app = block_uploader_app(solana_version, &default_args);
    let matches = cli_app.get_matches();

    let uploader_config = process_uploader_arguments(&matches);
    let cache_config = process_cache_arguments(&matches);

    env_logger::init();

    let app_config = Arc::new(Config::new());

    println!("Started encoder");

    let storage_config = LedgerStorageConfig {
        read_only: false,
        timeout: None,
        address: app_config.hbase_address.clone(),
        uploader_config: uploader_config.clone(),
        cache_config: cache_config.clone(),
    };
    let storage = LedgerStorage::new_with_config(storage_config).await;

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
        println!("Failed to read from stdin: {}", e);
        return Err(Box::new(e) as Box<dyn std::error::Error>);
    }

    println!("Encoding block");

    let parsed_json: Result<serde_json::Value, _> = serde_json::from_str(&buffer);
    let block_id = parsed_json.ok().and_then(|json| json["blockID"].as_u64());

    let slot = match block_id {
        Some(s) => s,
        None => {
            panic!("Invalid or missing blockID");
        }
    };

    let block: EncodedConfirmedBlock = serde_json::from_str(&buffer).unwrap();

    let options = BlockEncodingOptions {
        transaction_details: TransactionDetails::Full,
        show_rewards: true,
        max_supported_transaction_version: Some(0),
    };

    match convert_block(block, UiTransactionEncoding::Json, options) {
        Ok(versioned_block) => {
            // output_block(versioned_block).await?;

            match storage.upload_confirmed_block(slot, versioned_block).await {
                Ok(_) => (),
                Err(e) => panic!("Upload error: {}", e.to_string()),
            }
        }
        Err(e) => {
            panic!("Block conversion error: {}", e.to_string());
        }
    }

    Ok(())
}