use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum DataError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },

    #[error("Market not supported: {market}")]
    MarketNotSupported { market: String },

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("IO error: {0}")]
    Io(Arc<std::io::Error>),

    #[error("Config error: {0}")]
    Config(String),
}

impl From<std::io::Error> for DataError {
    fn from(e: std::io::Error) -> Self {
        DataError::Io(Arc::new(e))
    }
}
