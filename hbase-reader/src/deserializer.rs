use {
    crate::{
        hbase_error::Error,
        hbase::*,
    },
    solana_storage_utils::{
        compression::{decompress},
    },
    log::*,
};

pub(crate) fn deserialize_protobuf_or_bincode_cell_data<B, P>(
    row_data: RowDataSlice,
    table: &str,
    key: RowKey,
) -> Result<CellData<B, P>>
where
    B: serde::de::DeserializeOwned,
    P: prost::Message + Default,
{
    match deserialize_protobuf_cell_data(row_data, table, key.to_string()) {
        Ok(result) => {
            return Ok(CellData::Protobuf(result))
        },
        Err(err) => {
            match err {
                Error::ObjectNotFound(_) => {}
                _ => return Err(err),
            }
        },
    }
    deserialize_bincode_cell_data(row_data, table, key).map(CellData::Bincode)
}

pub(crate) fn deserialize_protobuf_cell_data<T>(
    row_data: RowDataSlice,
    table: &str,
    key: RowKey,
) -> Result<T>
where
    T: prost::Message + Default,
{
    let value = &row_data
        .iter()
        .find(|(name, _)| name == "x:proto")
        .ok_or_else(|| Error::ObjectNotFound(format!("{table}/{key}")))?
        .1;

    let data = decompress(value)?;
    T::decode(&data[..]).map_err(|err| {
        warn!("Failed to deserialize {}/{}: {}", table, key, err);
        Error::ObjectCorrupt(format!("{table}/{key}"))
    })
}

pub(crate) fn deserialize_bincode_cell_data<T>(
    row_data: RowDataSlice,
    table: &str,
    key: RowKey,
) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let value = &row_data
        .iter()
        .find(|(name, _)| name == "x:bin")
        .ok_or_else(|| Error::ObjectNotFound(format!("{table}/{key}")))?
        .1;

    let data = decompress(value)?;
    bincode::deserialize(&data).map_err(|err| {
        warn!("Failed to deserialize {}/{}: {}", table, key, err);
        Error::ObjectCorrupt(format!("{table}/{key}"))
    })
}