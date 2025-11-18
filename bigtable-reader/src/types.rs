use crate::error::Error;

pub type RowKey = String;
pub type RowData = Vec<(CellName, CellValue)>;
pub type RowDataSlice<'a> = &'a [(CellName, CellValue)];
pub type CellName = String;
pub type CellValue = Vec<u8>;
pub enum CellData<B, P> {
    Bincode(B),
    Protobuf(P),
}

pub type Result<T> = std::result::Result<T, Error>;