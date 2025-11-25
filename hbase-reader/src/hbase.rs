use {
    crate::{
        deserializer::{
            deserialize_bincode_cell_data,
            deserialize_protobuf_or_bincode_cell_data,
        },
        hbase_error::Error,
    },
    log::*,
    hbase_thrift::hbase::{
        HbaseSyncClient, THbaseSyncClient, TScan
    },
    thrift::{
        protocol::{
            TBinaryInputProtocol, TBinaryOutputProtocol,
        },
        transport::{
            TBufferedReadTransport, TBufferedWriteTransport,
            TTcpChannel
        },
    },
    std::collections::BTreeMap,
    std::convert::TryInto,
};


pub type Result<T> = std::result::Result<T, Error>;


pub type RowKey = String;
pub type RowData = Vec<(CellName, CellValue)>;
pub type RowDataSlice<'a> = &'a [(CellName, CellValue)];
pub type CellName = String;
pub type CellValue = Vec<u8>;
pub enum CellData<B, P> {
    Bincode(B),
    Protobuf(P),
}


type InputProtocol = TBinaryInputProtocol<TBufferedReadTransport<thrift::transport::ReadHalf<TTcpChannel>>>;
type OutputProtocol = TBinaryOutputProtocol<TBufferedWriteTransport<thrift::transport::WriteHalf<TTcpChannel>>>;

pub struct HBase {
    pub client: HbaseSyncClient<InputProtocol, OutputProtocol>,
    // timeout: Option<Duration>,
}

impl HBase {
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
        reversed: bool,
    ) -> Result<Vec<RowKey>> {
        if rows_limit == 0 {
            return Ok(vec![]);
        }

        debug!("Trying to get row keys in range {:?} - {:?} with limit {:?}", start_at, end_at, rows_limit);

        let mut scan = TScan::default();
        scan.start_row = start_at.map(|start_key| {
            start_key.into_bytes()
        });
        scan.stop_row = end_at.map(|end_key| {
            end_key.into_bytes()
        });
        scan.columns = None;
        scan.batch_size = Some(rows_limit as i32);
        scan.timestamp = None;
        scan.caching = rows_limit.try_into().ok();
        scan.reversed = Some(reversed);
        scan.filter_string = Some(b"KeyOnlyFilter()".to_vec());

        let scan_id = self.client.scanner_open_with_scan(
            table_name.as_bytes().to_vec(),
            scan,
            BTreeMap::new()
        )?;

        let mut results: Vec<(RowKey, RowData)> = Vec::new();
        let mut count = 0;
        loop {
            let row_results = self.client.scanner_get_list(
                scan_id,
                rows_limit as i32
            )?;
            if row_results.is_empty() {
                break;
            }
            for row_result in row_results {
                let row_key_bytes = row_result.row.unwrap();
                let row_key = String::from_utf8(row_key_bytes.clone()).unwrap();
                let mut column_values: RowData = Vec::new();
                for (key, column) in row_result.columns.unwrap_or_default() {
                    let column_value_bytes = column.value.unwrap_or_default();
                    column_values.push((String::from_utf8(key).unwrap(), column_value_bytes.into()));
                }
                results.push((row_key, column_values));
                count += 1;
                if count >= rows_limit {
                    break;
                }
            }
            if count >= rows_limit {
                break;
            }
        }

        self.client.scanner_close(scan_id)?;

        Ok(results.into_iter().map(|r| r.0).collect())
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

        debug!("Trying to get rows in range {:?} - {:?} with limit {:?}", start_at, end_at, rows_limit);

        let mut scan = TScan::default();

        scan.start_row = start_at.map(|start_key| {
            start_key.into_bytes()
        });
        scan.stop_row = end_at.map(|end_key| {
            end_key.into_bytes()
        });
        scan.columns = Some(vec!["x".as_bytes().to_vec()]);
        scan.batch_size = Some(rows_limit as i32);
        scan.timestamp = None;
        scan.caching = rows_limit.try_into().ok();
        scan.filter_string = Some(b"ColumnPaginationFilter(1,0)".to_vec());

        let scan_id = self.client.scanner_open_with_scan(
            table_name.as_bytes().to_vec(),
            scan,
            BTreeMap::new()
        )?;
        // ).unwrap_or_else(|err| {
        //     debug!("scanner_open_with_scan error: {:?}", err);
        //     std::process::exit(1);
        // });

        let mut results: Vec<(RowKey, RowData)> = Vec::new();
        let mut count = 0;

        loop {
            let row_results = self.client.scanner_get_list(
                scan_id,
                rows_limit as i32
            )?;
            // ).unwrap_or_else(|err| {
            //     debug!("scanner_get_list error: {:?}", err);
            //     std::process::exit(1);
            // });

            if row_results.is_empty() {
                break;
            }

            for row_result in row_results {
                let row_key_bytes = row_result.row.unwrap();
                let row_key = String::from_utf8(row_key_bytes.clone()).unwrap();
                let mut column_values: RowData = Vec::new();
                for (key, column) in row_result.columns.unwrap_or_default() {
                    let column_value_bytes = column.value.unwrap_or_default();
                    column_values.push((String::from_utf8(key).unwrap(), column_value_bytes.into()));
                }
                results.push((row_key, column_values));
                count += 1;
                if count >= rows_limit {
                    break;
                }
            }
            if count >= rows_limit {
                break;
            }
        }

        self.client.scanner_close(scan_id)?;

        Ok(results)
    }

    pub async fn get_single_row_data(
        &mut self,
        table_name: &str,
        row_key: RowKey,
    ) -> Result<RowData> {
        debug!("Trying to get row data with key {:?} from table {:?}", row_key, table_name);

        let row_result = self.client.get_row_with_columns(
            table_name.as_bytes().to_vec(),
            row_key.as_bytes().to_vec(),
            vec![b"x".to_vec()],
            BTreeMap::new()
        )?;

        let first_row_result = &row_result.into_iter()
            .next()
            .ok_or(Error::RowNotFound)?;

        let mut result_value: RowData = vec![];
        if let Some(cols) = &first_row_result.columns {
            for (col_name, cell) in cols {
                if let Some(value) = &cell.value {
                    result_value.push((String::from_utf8(col_name.to_vec()).unwrap().to_string(), value.to_vec()));
                }
            }
        }

        Ok(result_value)
    }

    pub async fn get_bincode_cell<T>(&mut self, table: &str, key: RowKey) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let row_data = self.get_single_row_data(table, key.clone()).await?;
        deserialize_bincode_cell_data(&row_data, table, key.to_string())
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

    pub async fn get_protobuf_or_bincode_cell_serialized<B, P>(
        &mut self,
        table: &str,
        key: RowKey,
    ) -> Result<RowData>
    where
        B: serde::de::DeserializeOwned,
        P: prost::Message + Default,
    {
        self.get_single_row_data(table, key.clone()).await
    }

    pub async fn get_last_row_key(&mut self, table_name: &str) -> Result<String> {
        let row_keys = self.get_row_keys(table_name, None, None, 1, true).await?;
        if let Some(last_row_key) = row_keys.first() {
            Ok(last_row_key.clone())
        } else {
            Err(Error::RowNotFound)
        }
    }
}


