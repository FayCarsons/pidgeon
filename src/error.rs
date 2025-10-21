use rustyline::error::ReadlineError;

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
    #[error("Server reported error '{0}'")]
    Server(#[from] axum::http::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
