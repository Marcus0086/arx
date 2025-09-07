use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format error: {0}")]
    Format(String),
}

// Convenient crate-wide result type
pub type Result<T> = std::result::Result<T, ArxError>;
