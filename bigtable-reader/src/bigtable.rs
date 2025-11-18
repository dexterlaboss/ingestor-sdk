// Primitives for reading/writing BigTable tables

use {
    solana_bigtable_shared::{
        access_token::{
            AccessToken,
        },
    },
    log::*,
    std::{
        time::{Duration, Instant},
    },
    tonic::{
        codegen::InterceptedService,
        Request,
    },
};

use solana_bigtable_shared::google::bigtable::v2::*;

pub use crate::types::*;
pub use crate::deserializer::*;
pub use crate::error::Error;
pub use crate::connection::*;

pub struct BigTable<F: FnMut(Request<()>) -> InterceptedRequestResult> {
    pub(crate) access_token: Option<AccessToken>,
    pub(crate) client: bigtable_client::BigtableClient<InterceptedService<tonic::transport::Channel, F>>,
    pub(crate) table_prefix: String,
    pub(crate) app_profile_id: String,
    pub(crate) timeout: Option<Duration>,
}

impl<F: FnMut(Request<()>) -> InterceptedRequestResult> BigTable<F> {
    async fn decode_read_rows_response(
        &self,
        mut rrr: tonic::codec::Streaming<ReadRowsResponse>,
    ) -> Result<Vec<(RowKey, RowData)>> {
        let mut rows: Vec<(RowKey, RowData)> = vec![];

        let mut row_key = None;
        let mut row_data = vec![];

        let mut cell_name = None;
        let mut cell_timestamp = 0;
        let mut cell_value = vec![];
        let mut cell_version_ok = true;
        let started = Instant::now();

        while let Some(res) = rrr.message().await? {
            if let Some(timeout) = self.timeout {
                if Instant::now().duration_since(started) > timeout {
                    return Err(Error::Timeout);
                }
            }
            for (i, mut chunk) in res.chunks.into_iter().enumerate() {
                // The comments for `read_rows_response::CellChunk` provide essential details for
                // understanding how the below decoding works...
                trace!("chunk {}: {:?}", i, chunk);

                // Starting a new row?
                if !chunk.row_key.is_empty() {
                    row_key = String::from_utf8(chunk.row_key).ok(); // Require UTF-8 for row keys
                }

                // Starting a new cell?
                if let Some(qualifier) = chunk.qualifier {
                    if let Some(cell_name) = cell_name {
                        row_data.push((cell_name, cell_value));
                        cell_value = vec![];
                    }
                    cell_name = String::from_utf8(qualifier).ok(); // Require UTF-8 for cell names
                    cell_timestamp = chunk.timestamp_micros;
                    cell_version_ok = true;
                } else {
                    // Continuing the existing cell.  Check if this is the start of another version of the cell
                    if chunk.timestamp_micros != 0 {
                        if chunk.timestamp_micros < cell_timestamp {
                            cell_version_ok = false; // ignore older versions of the cell
                        } else {
                            // newer version of the cell, remove the older cell
                            cell_version_ok = true;
                            cell_value = vec![];
                            cell_timestamp = chunk.timestamp_micros;
                        }
                    }
                }
                if cell_version_ok {
                    cell_value.append(&mut chunk.value);
                }

                // End of a row?
                if chunk.row_status.is_some() {
                    if let Some(read_rows_response::cell_chunk::RowStatus::CommitRow(_)) =
                        chunk.row_status
                    {
                        if let Some(cell_name) = cell_name {
                            row_data.push((cell_name, cell_value));
                        }

                        if let Some(row_key) = row_key {
                            rows.push((row_key, row_data))
                        }
                    }

                    row_key = None;
                    row_data = vec![];
                    cell_value = vec![];
                    cell_name = None;
                }
            }
        }
        Ok(rows)
    }

    async fn refresh_access_token(&self) {
        if let Some(ref access_token) = self.access_token {
            access_token.refresh().await;
        }
    }

    /// Get `table` row keys in lexical order.
    ///
    /// If `start_at` is provided, the row key listing will start with key.
    /// Otherwise the listing will start from the start of the table.
    ///
    /// If `end_at` is provided, the row key listing will end at the key. Otherwise it will
    /// continue until the `rows_limit` is reached or the end of the table, whichever comes first.
    /// If `rows_limit` is zero, this method will return an empty array.
    pub async fn get_row_keys(
        &mut self,
        table_name: &str,
        start_at: Option<RowKey>,
        end_at: Option<RowKey>,
        rows_limit: i64,
    ) -> Result<Vec<RowKey>> {
        if rows_limit == 0 {
            return Ok(vec![]);
        }
        self.refresh_access_token().await;
        let response = self
            .client
            .read_rows(ReadRowsRequest {
                table_name: format!("{}{}", self.table_prefix, table_name),
                app_profile_id: self.app_profile_id.clone(),
                rows_limit,
                rows: Some(RowSet {
                    row_keys: vec![],
                    row_ranges: vec![RowRange {
                        start_key: start_at.map(|row_key| {
                            row_range::StartKey::StartKeyClosed(row_key.into_bytes())
                        }),
                        end_key: end_at
                            .map(|row_key| row_range::EndKey::EndKeyClosed(row_key.into_bytes())),
                    }],
                }),
                filter: Some(RowFilter {
                    filter: Some(row_filter::Filter::Chain(row_filter::Chain {
                        filters: vec![
                            RowFilter {
                                // Return minimal number of cells
                                filter: Some(row_filter::Filter::CellsPerRowLimitFilter(1)),
                            },
                            RowFilter {
                                // Only return the latest version of each cell
                                filter: Some(row_filter::Filter::CellsPerColumnLimitFilter(1)),
                            },
                            RowFilter {
                                // Strip the cell values
                                filter: Some(row_filter::Filter::StripValueTransformer(true)),
                            },
                        ],
                    })),
                }),
            })
            .await?
            .into_inner();

        let rows = self.decode_read_rows_response(response).await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// Check whether a row key exists in a `table`
    pub async fn row_key_exists(&mut self, table_name: &str, row_key: RowKey) -> Result<bool> {
        self.refresh_access_token().await;

        let response = self
            .client
            .read_rows(ReadRowsRequest {
                table_name: format!("{}{}", self.table_prefix, table_name),
                app_profile_id: self.app_profile_id.clone(),
                rows_limit: 1,
                rows: Some(RowSet {
                    row_keys: vec![row_key.into_bytes()],
                    row_ranges: vec![],
                }),
                filter: Some(RowFilter {
                    filter: Some(row_filter::Filter::StripValueTransformer(true)),
                }),
            })
            .await?
            .into_inner();

        let rows = self.decode_read_rows_response(response).await?;
        Ok(!rows.is_empty())
    }

    /// Get latest data from `table`.
    ///
    /// All column families are accepted, and only the latest version of each column cell will be
    /// returned.
    ///
    /// If `start_at` is provided, the row key listing will start with key, or the next key in the
    /// table if the explicit key does not exist. Otherwise the listing will start from the start
    /// of the table.
    ///
    /// If `end_at` is provided, the row key listing will end at the key. Otherwise it will
    /// continue until the `rows_limit` is reached or the end of the table, whichever comes first.
    /// If `rows_limit` is zero, this method will return an empty array.
    pub async fn get_row_data(
        &mut self,
        table_name: &str,
        start_at: Option<RowKey>,
        end_at: Option<RowKey>,
        rows_limit: i64,
    ) -> Result<Vec<(RowKey, RowData)>> {
        if rows_limit == 0 {
            return Ok(vec![]);
        }
        self.refresh_access_token().await;
        let response = self
            .client
            .read_rows(ReadRowsRequest {
                table_name: format!("{}{}", self.table_prefix, table_name),
                app_profile_id: self.app_profile_id.clone(),
                rows_limit,
                rows: Some(RowSet {
                    row_keys: vec![],
                    row_ranges: vec![RowRange {
                        start_key: start_at.map(|row_key| {
                            row_range::StartKey::StartKeyClosed(row_key.into_bytes())
                        }),
                        end_key: end_at
                            .map(|row_key| row_range::EndKey::EndKeyClosed(row_key.into_bytes())),
                    }],
                }),
                filter: Some(RowFilter {
                    // Only return the latest version of each cell
                    filter: Some(row_filter::Filter::CellsPerColumnLimitFilter(1)),
                }),
            })
            .await?
            .into_inner();

        self.decode_read_rows_response(response).await
    }

    /// Get latest data from multiple rows of `table`, if those rows exist.
    pub async fn get_multi_row_data(
        &mut self,
        table_name: &str,
        row_keys: &[RowKey],
    ) -> Result<Vec<(RowKey, RowData)>> {
        self.refresh_access_token().await;

        let response = self
            .client
            .read_rows(ReadRowsRequest {
                table_name: format!("{}{}", self.table_prefix, table_name),
                app_profile_id: self.app_profile_id.clone(),
                rows_limit: 0, // return all existing rows
                rows: Some(RowSet {
                    row_keys: row_keys
                        .iter()
                        .map(|k| k.as_bytes().to_vec())
                        .collect::<Vec<_>>(),
                    row_ranges: vec![],
                }),
                filter: Some(RowFilter {
                    // Only return the latest version of each cell
                    filter: Some(row_filter::Filter::CellsPerColumnLimitFilter(1)),
                }),
            })
            .await?
            .into_inner();

        self.decode_read_rows_response(response).await
    }

    /// Get latest data from a single row of `table`, if that row exists. Returns an error if that
    /// row does not exist.
    ///
    /// All column families are accepted, and only the latest version of each column cell will be
    /// returned.
    pub async fn get_single_row_data(
        &mut self,
        table_name: &str,
        row_key: RowKey,
    ) -> Result<RowData> {
        self.refresh_access_token().await;

        let response = self
            .client
            .read_rows(ReadRowsRequest {
                table_name: format!("{}{}", self.table_prefix, table_name),
                app_profile_id: self.app_profile_id.clone(),
                rows_limit: 1,
                rows: Some(RowSet {
                    row_keys: vec![row_key.into_bytes()],
                    row_ranges: vec![],
                }),
                filter: Some(RowFilter {
                    // Only return the latest version of each cell
                    filter: Some(row_filter::Filter::CellsPerColumnLimitFilter(1)),
                }),
            })
            .await?
            .into_inner();

        let rows = self.decode_read_rows_response(response).await?;
        rows.into_iter()
            .next()
            .map(|r| r.1)
            .ok_or(Error::RowNotFound)
    }

    pub async fn get_bincode_cell<T>(&mut self, table: &str, key: RowKey) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let row_data = self.get_single_row_data(table, key.clone()).await?;
        deserialize_bincode_cell_data(&row_data, table, key.to_string())
    }

    pub async fn get_bincode_cells<T>(
        &mut self,
        table: &str,
        keys: &[RowKey],
    ) -> Result<Vec<(RowKey, Result<T>)>>
    where
        T: serde::de::DeserializeOwned,
    {
        Ok(self
            .get_multi_row_data(table, keys)
            .await?
            .into_iter()
            .map(|(key, row_data)| {
                let key_str = key.to_string();
                (
                    key,
                    deserialize_bincode_cell_data(&row_data, table, key_str),
                )
            })
            .collect())
    }

    pub async fn get_protobuf_or_bincode_cell<B, P>(
        &mut self,
        table: &str,
        key: RowKey,
    ) -> Result<CellData<B, P>>
    where
        B: serde::de::DeserializeOwned,
        P: prost::Message + Default,
    {
        let row_data = self.get_single_row_data(table, key.clone()).await?;
        deserialize_protobuf_or_bincode_cell_data(&row_data, table, key)
    }

    pub async fn get_protobuf_or_bincode_cells<'a, B, P>(
        &mut self,
        table: &'a str,
        row_keys: impl IntoIterator<Item = RowKey>,
    ) -> Result<impl Iterator<Item = (RowKey, CellData<B, P>)> + 'a>
    where
        B: serde::de::DeserializeOwned,
        P: prost::Message + Default,
    {
        Ok(self
            .get_multi_row_data(
                table,
                row_keys.into_iter().collect::<Vec<RowKey>>().as_slice(),
            )
            .await?
            .into_iter()
            .map(|(key, row_data)| {
                let key_str = key.to_string();
                (
                    key,
                    deserialize_protobuf_or_bincode_cell_data(&row_data, table, key_str).unwrap(),
                )
            }))
    }
}



#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::StoredConfirmedBlock,
        prost::Message,
        solana_hash::Hash,
        solana_keypair::Keypair,
        solana_message::v0::LoadedAddresses,
        solana_storage_proto::convert::generated,
        solana_system_transaction as system_transaction,
        solana_transaction::versioned::VersionedTransaction,
        solana_transaction_context::TransactionReturnData,
        solana_transaction_status::{
            ConfirmedBlock, TransactionStatusMeta, TransactionWithStatusMeta,
            VersionedTransactionWithStatusMeta,
        },
        solana_storage_utils::compression::compress_best,
        std::convert::TryInto,
    };

    fn confirmed_block_into_protobuf(confirmed_block: ConfirmedBlock) -> generated::ConfirmedBlock {
        let ConfirmedBlock {
            previous_blockhash,
            blockhash,
            parent_slot,
            transactions,
            rewards,
            num_partitions,
            block_time,
            block_height,
        } = confirmed_block;

        generated::ConfirmedBlock {
            previous_blockhash,
            blockhash,
            parent_slot,
            transactions: transactions.into_iter().map(|tx| tx.into()).collect(),
            rewards: rewards.into_iter().map(|r| r.into()).collect(),
            num_partitions: num_partitions
                .map(|num_partitions| generated::NumPartitions { num_partitions }),
            block_time: block_time.map(|timestamp| generated::UnixTimestamp { timestamp }),
            block_height: block_height.map(|block_height| generated::BlockHeight { block_height }),
        }
    }

    #[test]
    fn test_deserialize_protobuf_or_bincode_cell_data() {
        let from = Keypair::new();
        let recipient = solana_pubkey::new_rand();
        let transaction = system_transaction::transfer(&from, &recipient, 42, Hash::default());
        let with_meta = TransactionWithStatusMeta::Complete(VersionedTransactionWithStatusMeta {
            transaction: VersionedTransaction::from(transaction),
            meta: TransactionStatusMeta {
                status: Ok(()),
                fee: 1,
                pre_balances: vec![43, 0, 1],
                post_balances: vec![0, 42, 1],
                inner_instructions: Some(vec![]),
                log_messages: Some(vec![]),
                pre_token_balances: Some(vec![]),
                post_token_balances: Some(vec![]),
                rewards: Some(vec![]),
                loaded_addresses: LoadedAddresses::default(),
                return_data: Some(TransactionReturnData::default()),
                compute_units_consumed: Some(1234),
            },
        });
        let expected_block = ConfirmedBlock {
            transactions: vec![with_meta],
            parent_slot: 1,
            blockhash: Hash::default().to_string(),
            previous_blockhash: Hash::default().to_string(),
            rewards: vec![],
            num_partitions: None,
            block_time: Some(1_234_567_890),
            block_height: Some(1),
        };
        let bincode_block = compress_best(
            &bincode::serialize::<StoredConfirmedBlock>(&expected_block.clone().into()).unwrap(),
        )
            .unwrap();

        let protobuf_block = confirmed_block_into_protobuf(expected_block.clone());
        let mut buf = Vec::with_capacity(protobuf_block.encoded_len());
        protobuf_block.encode(&mut buf).unwrap();
        let protobuf_block = compress_best(&buf).unwrap();

        let deserialized = deserialize_protobuf_or_bincode_cell_data::<
            StoredConfirmedBlock,
            generated::ConfirmedBlock,
        >(
            &[("proto".to_string(), protobuf_block.clone())],
            "",
            "".to_string(),
        )
            .unwrap();
        if let CellData::Protobuf(protobuf_block) = deserialized {
            assert_eq!(expected_block, protobuf_block.try_into().unwrap());
        } else {
            panic!("deserialization should produce CellData::Protobuf");
        }

        let deserialized = deserialize_protobuf_or_bincode_cell_data::<
            StoredConfirmedBlock,
            generated::ConfirmedBlock,
        >(
            &[("bin".to_string(), bincode_block.clone())],
            "",
            "".to_string(),
        )
            .unwrap();
        if let CellData::Bincode(bincode_block) = deserialized {
            let mut block = expected_block;
            if let TransactionWithStatusMeta::Complete(VersionedTransactionWithStatusMeta {
                                                           meta,
                                                           ..
                                                       }) = &mut block.transactions[0]
            {
                meta.inner_instructions = None; // Legacy bincode implementation does not support inner_instructions
                meta.log_messages = None; // Legacy bincode implementation does not support log_messages
                meta.pre_token_balances = None; // Legacy bincode implementation does not support token balances
                meta.post_token_balances = None; // Legacy bincode implementation does not support token balances
                meta.rewards = None; // Legacy bincode implementation does not support rewards
                meta.return_data = None; // Legacy bincode implementation does not support return data
                meta.compute_units_consumed = None; // Legacy bincode implementation does not support CU consumed
            }
            assert_eq!(block, bincode_block.into());
        } else {
            panic!("deserialization should produce CellData::Bincode");
        }

        let result = deserialize_protobuf_or_bincode_cell_data::<
            StoredConfirmedBlock,
            generated::ConfirmedBlock,
        >(&[("proto".to_string(), bincode_block)], "", "".to_string());
        assert!(result.is_err());

        let result = deserialize_protobuf_or_bincode_cell_data::<
            StoredConfirmedBlock,
            generated::ConfirmedBlock,
        >(
            &[("proto".to_string(), vec![1, 2, 3, 4])],
            "",
            "".to_string(),
        );
        assert!(result.is_err());

        let result = deserialize_protobuf_or_bincode_cell_data::<
            StoredConfirmedBlock,
            generated::ConfirmedBlock,
        >(&[("bin".to_string(), protobuf_block)], "", "".to_string());
        assert!(result.is_err());

        let result = deserialize_protobuf_or_bincode_cell_data::<
            StoredConfirmedBlock,
            generated::ConfirmedBlock,
        >(&[("bin".to_string(), vec![1, 2, 3, 4])], "", "".to_string());
        assert!(result.is_err());
    }
}
