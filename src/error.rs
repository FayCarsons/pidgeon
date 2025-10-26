use rustyline::error::ReadlineError;
use tokio_util::codec::LinesCodecError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Serial Error: {0}")]
    Serial(#[from] tokio_serial::Error),
    #[error("Crow not found")]
    NotFound,
    #[error("IO error '{0}'")]
    IO(#[from] std::io::Error),
    #[error("Repl error '{0}'")]
    Repl(#[from] ReadlineError),
    #[error("Unexpected response from crow: '{0}'")]
    Codec(#[from] LinesCodecError),
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Serialization failed: '{0}'")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
