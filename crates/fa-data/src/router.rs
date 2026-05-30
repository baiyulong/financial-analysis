use async_trait::async_trait;
use fa_core::{DataError, DataProvider, Market, Quote, Symbol, OHLCV, Period};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Automatically selects provider by market; falls back on failure.
pub struct ProviderRouter {
    providers: Vec<Arc<dyn DataProvider>>,
    max_retries: u32,
}

impl ProviderRouter {
    pub fn new(providers: Vec<Arc<dyn DataProvider>>) -> Self {
        Self { providers, max_retries: 3 }
    }

    fn providers_for(&self, market: &Market) -> Vec<&Arc<dyn DataProvider>> {
        self.providers.iter().filter(|p| p.supports(market)).collect()
    }
}

#[async_trait]
impl DataProvider for ProviderRouter {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        let candidates = self.providers_for(&symbol.market);
        if candidates.is_empty() {
            return Err(DataError::MarketNotSupported { market: symbol.market.to_string() });
        }

        let mut last_err = DataError::Network("no providers tried".into());

        for provider in candidates {
            let mut delay = Duration::from_millis(100);
            for attempt in 0..self.max_retries {
                match provider.fetch_quote(symbol).await {
                    Ok(quote) => return Ok(quote),
                    Err(DataError::RateLimited { retry_after }) => {
                        last_err = DataError::RateLimited { retry_after };
                        if attempt + 1 < self.max_retries {
                            sleep(Duration::from_secs(retry_after)).await;
                        }
                    }
                    Err(e) => {
                        last_err = e;
                        if attempt + 1 < self.max_retries {
                            sleep(delay).await;
                            delay *= 2;
                        }
                    }
                }
            }
        }

        Err(last_err)
    }

    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError> {
        let candidates = self.providers_for(&symbol.market);
        for provider in candidates {
            match provider.fetch_ohlcv(symbol, period).await {
                Ok(data) => return Ok(data),
                Err(_) => continue,
            }
        }
        Err(DataError::Network("all providers failed for OHLCV".into()))
    }

    fn name(&self) -> &'static str { "ProviderRouter" }

    fn supports(&self, market: &Market) -> bool {
        self.providers.iter().any(|p| p.supports(market))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use fa_core::{Market, Quote, Symbol};
    use rust_decimal::Decimal;
    use std::sync::Arc;

    struct AlwaysFailProvider;
    struct AlwaysSucceedProvider;

    #[async_trait]
    impl DataProvider for AlwaysFailProvider {
        async fn fetch_quote(&self, _: &Symbol) -> Result<Quote, DataError> {
            Err(DataError::Network("network error".into()))
        }
        async fn fetch_ohlcv(&self, _: &Symbol, _: Period) -> Result<Vec<OHLCV>, DataError> {
            Err(DataError::Network("network error".into()))
        }
        fn name(&self) -> &'static str { "FailProvider" }
        fn supports(&self, _: &Market) -> bool { true }
    }

    #[async_trait]
    impl DataProvider for AlwaysSucceedProvider {
        async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
            Ok(Quote {
                symbol: symbol.clone(),
                price: Decimal::from(100),
                change: Decimal::ZERO,
                change_pct: Decimal::ZERO,
                open: None, high: None, low: None, volume: None,
                market_cap: None, pe_ratio: None,
                week_52_high: None, week_52_low: None,
                name: None, timestamp: Utc::now(),
            })
        }
        async fn fetch_ohlcv(&self, _: &Symbol, _: Period) -> Result<Vec<OHLCV>, DataError> {
            Ok(vec![])
        }
        fn name(&self) -> &'static str { "SucceedProvider" }
        fn supports(&self, _: &Market) -> bool { true }
    }

    #[tokio::test]
    async fn test_router_uses_fallback() {
        let router = ProviderRouter {
            providers: vec![
                Arc::new(AlwaysFailProvider),
                Arc::new(AlwaysSucceedProvider),
            ],
            max_retries: 1, // skip retry delay in tests
        };
        let symbol = Symbol::new("AAPL", Market::USStock);
        let quote = router.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, Decimal::from(100));
    }

    #[tokio::test]
    async fn test_router_no_provider_for_market() {
        struct NoSupportProvider;
        #[async_trait]
        impl DataProvider for NoSupportProvider {
            async fn fetch_quote(&self, _: &Symbol) -> Result<Quote, DataError> { unimplemented!() }
            async fn fetch_ohlcv(&self, _: &Symbol, _: Period) -> Result<Vec<OHLCV>, DataError> { unimplemented!() }
            fn name(&self) -> &'static str { "NoSupport" }
            fn supports(&self, _: &Market) -> bool { false }
        }
        let router = ProviderRouter::new(vec![Arc::new(NoSupportProvider)]);
        let symbol = Symbol::new("AAPL", Market::USStock);
        assert!(router.fetch_quote(&symbol).await.is_err());
    }
}
