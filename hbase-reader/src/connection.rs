use {
    crate::{
        hbase::{HBase, Result},
    },
    log::*,
    std::{
        time::{
            Duration,
        },
    },
    hbase_thrift::hbase::{
        HbaseSyncClient,
    },
    thrift::{
        protocol::{
            TBinaryInputProtocol, TBinaryOutputProtocol,
        },
        transport::{TBufferedReadTransport, TBufferedWriteTransport, TIoChannel, TTcpChannel},
    },
};

#[derive(Clone, Debug)]
pub struct HBaseConnection {
    address: String,
    // timeout: Option<Duration>,
}

impl HBaseConnection {
    pub async fn new(
        address: &str,
        _read_only: bool,
        _timeout: Option<Duration>,
    ) -> Result<Self> {
        debug!("Creating HBase connection instance");

        Ok(Self {
            address: address.to_string(),
            // timeout,
        })
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
            // timeout: self.timeout,
        }
    }
}