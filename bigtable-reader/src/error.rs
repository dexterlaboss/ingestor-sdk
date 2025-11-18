use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("AccessToken: {0}")]
    AccessToken(String),

    #[error("Certificate: {0}")]
    Certificate(String),

    #[error("I/O: {0}")]
    Io(std::io::Error),

    #[error("Transport: {0}")]
    Transport(tonic::transport::Error),

    #[error("Invalid URI {0}: {1}")]
    InvalidUri(String, String),

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

    #[error("RPC: {0}")]
    Rpc(tonic::Status),

    #[error("Timeout")]
    Timeout,
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl std::convert::From<tonic::transport::Error> for Error {
    fn from(err: tonic::transport::Error) -> Self {
        Self::Transport(err)
    }
}

impl std::convert::From<tonic::Status> for Error {
    fn from(err: tonic::Status) -> Self {
        Self::Rpc(err)
    }
}