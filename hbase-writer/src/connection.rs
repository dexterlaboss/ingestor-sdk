use {
    crate::{
        hbase::{
            HBase,
            RowKey,
            Result,
        }
    },
    backoff::{future::retry, ExponentialBackoff},
    std::{
        time::{
            Duration,
        },
    },
    hbase_thrift::hbase::{HbaseSyncClient},
    thrift::{
        protocol::{
            TBinaryInputProtocol, TBinaryOutputProtocol,
        },
        transport::{TBufferedReadTransport, TBufferedWriteTransport, TIoChannel, TTcpChannel},
    },
    log::{debug, info},
};

#[derive(Clone)]
pub struct HBaseConnection {
    address: String,
    timeout: Option<Duration>,
}

impl HBaseConnection {
    pub async fn new(
        address: &str,
        _read_only: bool,
        timeout: Option<Duration>,
    ) -> Self {
        info!("Connecting to HBase at address {}", address.to_string());

        Self {
            address: address.to_string(),
            timeout,
        }
    }

    pub fn client(&self) -> HBase {
        let mut channel = TTcpChannel::new();

        channel.open(self.address.clone()).unwrap();

        let (input_chan, output_chan) = channel.split().unwrap();

        let input_prot = TBinaryInputProtocol::new(
            TBufferedReadTransport::new(input_chan),
            true
        );
        let output_prot = TBinaryOutputProtocol::new(
            TBufferedWriteTransport::new(output_chan),
            true
        );

        let client = HbaseSyncClient::new(
            input_prot,
            output_prot
        );

        HBase {
            client,
            timeout: self.timeout,
        }
    }

    pub async fn put_bincode_cells_with_retry<T>(
        &self,
        table: &str,
        cells: &[(RowKey, T)],
        use_compression: bool,
        use_wal: bool,
    ) -> Result<usize>
    where
        T: serde::ser::Serialize,
    {
        retry(ExponentialBackoff::default(), || async {
            let mut client = self.client();
            Ok(client.put_bincode_cells(table, cells, use_compression, use_wal).await?)
        })
            .await
    }

    pub async fn put_protobuf_cells_with_retry<T>(
        &self,
        table: &str,
        cells: &[(RowKey, T)],
        use_compression: bool,
        use_wal: bool,
    ) -> Result<usize>
    where
        T: prost::Message,
    {
        retry(ExponentialBackoff::default(), || async {
            let mut client = self.client();
            Ok(client.put_protobuf_cells(table, cells, use_compression, use_wal).await?)
        })
            .await
    }
}