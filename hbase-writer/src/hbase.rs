use {
    solana_storage_writer::{
        compression::{compress_best, compress, CompressionMethod},
    },
    log::*,
    std::{
        time::{
            Duration,
        },
    },
    thiserror::Error,
    hbase_thrift::hbase::{
        BatchMutation, HbaseSyncClient, THbaseSyncClient,
    },
    hbase_thrift::{
        MutationBuilder
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
};

pub type RowKey = String;
pub type RowData = Vec<(CellName, CellValue)>;
pub type RowDataSlice<'a> = &'a [(CellName, CellValue)];
pub type CellName = String;
pub type CellValue = Vec<u8>;
pub enum CellData<B, P> {
    Bincode(B),
    Protobuf(P),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(std::io::Error),

    #[error("Row not found")]
    RowNotFound,

    #[error("Row write failed")]
    RowWriteFailed,

    #[error("Row delete failed")]
    RowDeleteFailed,

    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    #[error("Object is corrupt: {0}")]
    ObjectCorrupt(String),

    #[error("Timeout")]
    Timeout,

    #[error("Thrift")]
    Thrift(thrift::Error),
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl std::convert::From<thrift::Error> for Error {
    fn from(err: thrift::Error) -> Self {
        Self::Thrift(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;


type InputTransport = TBufferedReadTransport<thrift::transport::ReadHalf<TTcpChannel>>;
type OutputTransport = TBufferedWriteTransport<thrift::transport::WriteHalf<TTcpChannel>>;

type InputProtocol = TBinaryInputProtocol<InputTransport>;
type OutputProtocol = TBinaryOutputProtocol<OutputTransport>;

pub struct HBase {
    pub client: HbaseSyncClient<InputProtocol, OutputProtocol>,
    pub timeout: Option<Duration>,
}

impl HBase {
    pub async fn put_bincode_cells<T>(
        &mut self,
        table: &str,
        cells: &[(RowKey, T)],
        use_compression: bool,
        use_wal: bool,
    ) -> Result<usize>
    where
        T: serde::ser::Serialize,
    {
        let mut bytes_written = 0;
        let mut new_row_data = vec![];
        for (row_key, data) in cells {
            let serialized_data = bincode::serialize(&data).unwrap();

            let data = if use_compression {
                compress_best(&serialized_data)?
            } else {
                compress(CompressionMethod::NoCompression, &serialized_data)?
            };

            bytes_written += data.len();
            new_row_data.push((row_key, vec![("bin".to_string(), data)]));
        }

        self.put_row_data(table, "x", &new_row_data, use_wal).await?;
        Ok(bytes_written)
    }

    pub async fn put_protobuf_cells<T>(
        &mut self,
        table: &str,
        cells: &[(RowKey, T)],
        use_compression: bool,
        use_wal: bool,
    ) -> Result<usize>
    where
        T: prost::Message,
    {
        let mut bytes_written = 0;
        let mut new_row_data = vec![];
        for (row_key, data) in cells {
            let mut buf = Vec::with_capacity(data.encoded_len());
            data.encode(&mut buf).unwrap();

            let data = if use_compression {
                compress_best(&buf)?
            } else {
                compress(CompressionMethod::NoCompression, &buf)?
            };

            bytes_written += data.len();
            new_row_data.push((row_key, vec![("proto".to_string(), data)]));
        }

        self.put_row_data(table, "x", &new_row_data, use_wal).await?;
        Ok(bytes_written)
    }

    async fn put_row_data(
        &mut self,
        table_name: &str,
        family_name: &str,
        row_data: &[(&RowKey, RowData)],
        use_wal: bool,
    ) -> Result<()> {
        let mut mutation_batches = Vec::new();
        for (row_key, cell_data) in row_data {
            let mut mutations = Vec::new();
            for (cell_name, cell_value) in cell_data {
                let mut mutation_builder = MutationBuilder::default();
                mutation_builder.column(family_name, cell_name);
                mutation_builder.value(cell_value.clone());
                mutation_builder.write_to_wal(use_wal);
                mutations.push(mutation_builder.build());
            }
            mutation_batches.push(BatchMutation::new(Some(row_key.as_bytes().to_vec()), mutations));
        }

        self.client.mutate_rows(table_name.as_bytes().to_vec(), mutation_batches, Default::default())?;

        Ok(())
    }
}

