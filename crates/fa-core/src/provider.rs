use crate::{DataError, Market, Period, Quote, Symbol, OHLCV};
use async_trait::async_trait;

#[async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError>;
    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError>;

    /// Fetch OHLCV with an extended date range, keeping the same bar granularity.
    /// Used for "load more history" at the left edge of the K-line chart.
    /// The default implementation delegates to `fetch_ohlcv`; providers like AkShare already
    /// return maximum history so they don't need to override this.
    async fn fetch_ohlcv_extended(
        &self,
        symbol: &Symbol,
        period: Period,
    ) -> Result<Vec<OHLCV>, DataError> {
        self.fetch_ohlcv(symbol, period).await
    }

    fn name(&self) -> &'static str;
    fn supports(&self, market: &Market) -> bool;
}
