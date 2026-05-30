use async_trait::async_trait;
use crate::{DataError, Market, OHLCV, Period, Quote, Symbol};

#[async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError>;
    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError>;
    fn name(&self) -> &'static str;
    fn supports(&self, market: &Market) -> bool;
}
