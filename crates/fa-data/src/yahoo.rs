// Yahoo Finance data provider
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fa_core::{DataError, DataProvider, Market, Quote, Symbol, OHLCV, Period};
use rust_decimal::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::cache::InMemoryCache;

const DEFAULT_BASE_URL: &str = "https://query1.finance.yahoo.com";

pub struct YahooFinanceProvider {
    client: reqwest::Client,
    base_url: String,
    cache: Arc<Mutex<InMemoryCache<Quote>>>,
}

impl YahooFinanceProvider {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            cache: Arc::new(Mutex::new(InMemoryCache::new(
                std::time::Duration::from_secs(60),
            ))),
        }
    }
}

// --- Deserialization structs ---

#[derive(Deserialize)]
struct YahooResponse {
    #[serde(rename = "quoteResponse")]
    quote_response: QuoteResponse,
}

#[derive(Deserialize)]
struct QuoteResponse {
    result: Vec<YahooQuoteResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct YahooQuoteResult {
    #[allow(dead_code)]
    symbol: String,
    long_name: Option<String>,
    regular_market_price: f64,
    regular_market_change: f64,
    regular_market_change_percent: f64,
    regular_market_open: Option<f64>,
    regular_market_day_high: Option<f64>,
    regular_market_day_low: Option<f64>,
    regular_market_volume: Option<u64>,
    market_cap: Option<f64>,
    trailing_pe: Option<f64>,
    fifty_two_week_high: Option<f64>,
    fifty_two_week_low: Option<f64>,
    regular_market_time: Option<i64>,
}

fn f64_to_dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

fn into_quote(r: YahooQuoteResult, symbol: &Symbol) -> Result<Quote, DataError> {
    let ts = r.regular_market_time
        .and_then(|t| DateTime::from_timestamp(t, 0))
        .unwrap_or_else(Utc::now);

    Ok(Quote {
        symbol: symbol.clone(),
        price: f64_to_dec(r.regular_market_price),
        change: f64_to_dec(r.regular_market_change),
        change_pct: f64_to_dec(r.regular_market_change_percent),
        open: r.regular_market_open.map(f64_to_dec),
        high: r.regular_market_day_high.map(f64_to_dec),
        low: r.regular_market_day_low.map(f64_to_dec),
        volume: r.regular_market_volume,
        market_cap: r.market_cap.map(f64_to_dec),
        pe_ratio: r.trailing_pe.map(f64_to_dec),
        week_52_high: r.fifty_two_week_high.map(f64_to_dec),
        week_52_low: r.fifty_two_week_low.map(f64_to_dec),
        name: r.long_name,
        timestamp: ts,
    })
}

#[async_trait]
impl DataProvider for YahooFinanceProvider {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        let ticker = symbol.yahoo_ticker();

        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&ticker) {
                return Ok(cached);
            }
        }

        let url = format!(
            "{}/v7/finance/quote?symbols={}&fields=regularMarketPrice,regularMarketChange,\
             regularMarketChangePercent,regularMarketOpen,regularMarketDayHigh,\
             regularMarketDayLow,regularMarketVolume,marketCap,trailingPE,\
             fiftyTwoWeekHigh,fiftyTwoWeekLow,longName,regularMarketTime",
            self.base_url, ticker
        );

        let resp = self.client.get(&url).send().await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DataError::RateLimited { retry_after: 60 });
        }

        let yahoo: YahooResponse = resp.json().await
            .map_err(|e| DataError::Parse(e.to_string()))?;

        let result = yahoo.quote_response.result.into_iter().next()
            .ok_or_else(|| DataError::SymbolNotFound { symbol: symbol.code.clone() })?;

        let quote = into_quote(result, symbol)?;

        {
            let mut cache = self.cache.lock().await;
            cache.set(ticker, quote.clone());
        }

        Ok(quote)
    }

    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError> {
        let ticker = symbol.yahoo_ticker();
        let url = format!(
            "{}/v8/finance/chart/{}?interval={}&range={}",
            self.base_url, ticker,
            period.yahoo_interval(),
            period.yahoo_range()
        );

        let resp = self.client.get(&url).send().await
            .map_err(|e| DataError::Network(e.to_string()))?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| DataError::Parse(e.to_string()))?;

        parse_ohlcv_response(&json, symbol)
    }

    fn name(&self) -> &'static str { "Yahoo Finance" }

    fn supports(&self, _market: &Market) -> bool {
        true
    }
}

fn parse_ohlcv_response(json: &serde_json::Value, symbol: &Symbol) -> Result<Vec<OHLCV>, DataError> {
    let result = &json["chart"]["result"][0];
    let timestamps = result["timestamp"].as_array()
        .ok_or_else(|| DataError::Parse("missing timestamps".into()))?;
    let quote = &result["indicators"]["quote"][0];

    let opens  = quote["open"].as_array().ok_or_else(|| DataError::Parse("missing open".into()))?;
    let highs  = quote["high"].as_array().ok_or_else(|| DataError::Parse("missing high".into()))?;
    let lows   = quote["low"].as_array().ok_or_else(|| DataError::Parse("missing low".into()))?;
    let closes = quote["close"].as_array().ok_or_else(|| DataError::Parse("missing close".into()))?;
    let vols   = quote["volume"].as_array().ok_or_else(|| DataError::Parse("missing volume".into()))?;

    let bars = timestamps.iter().enumerate()
        .filter_map(|(i, ts)| {
            let ts = ts.as_i64()?;
            let dt = DateTime::from_timestamp(ts, 0)?;
            Some(OHLCV {
                symbol: symbol.clone(),
                timestamp: dt,
                open:   f64_to_dec(opens[i].as_f64()?),
                high:   f64_to_dec(highs[i].as_f64()?),
                low:    f64_to_dec(lows[i].as_f64()?),
                close:  f64_to_dec(closes[i].as_f64()?),
                volume: vols[i].as_u64().unwrap_or(0),
            })
        })
        .collect();

    Ok(bars)
}



#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use mockito::Server;

    #[tokio::test]
    async fn test_fetch_quote_success() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/yahoo_quote_aapl.json");

        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(fixture)
            .create_async()
            .await;

        let provider = YahooFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("AAPL", Market::USStock);
        let quote = provider.fetch_quote(&symbol).await.unwrap();

        assert_eq!(quote.symbol.code, "AAPL");
        assert_eq!(quote.price, rust_decimal_macros::dec!(185.20));
        assert_eq!(quote.name.as_deref(), Some("Apple Inc."));
        assert!(quote.volume.unwrap() > 0);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_quote_cache_hit() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/yahoo_quote_aapl.json");
        // Only ONE mock — second call should use cache
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(fixture)
            .expect(1)
            .create_async()
            .await;

        let provider = YahooFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("AAPL", Market::USStock);
        provider.fetch_quote(&symbol).await.unwrap();
        provider.fetch_quote(&symbol).await.unwrap(); // cache hit
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_supports_markets() {
        let p = YahooFinanceProvider::new();
        assert!(p.supports(&fa_core::Market::USStock));
        assert!(p.supports(&fa_core::Market::HKStock));
        assert!(p.supports(&fa_core::Market::AShare));
        assert!(p.supports(&fa_core::Market::Crypto));
    }
}
