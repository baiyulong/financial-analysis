// crates/fa-data/src/zhitu.rs
use async_trait::async_trait;
use chrono::{FixedOffset, NaiveDateTime, TimeZone, Utc};
use fa_core::{zhitu_adjust, DataError, DataProvider, Market, Period, Quote, Symbol, OHLCV};
use rust_decimal::prelude::*;
use serde::Deserialize;

const DEFAULT_BASE_URL: &str = "https://api.zhituapi.com";

pub struct ZhituProvider {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl ZhituProvider {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
            token: token.into(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Returns true if the symbol represents a market index (e.g. sh000001, sz399001).
    /// ZhituAPI only supports individual stocks, not indices.
    /// Bare codes like `000001` are ambiguous (could be stock or index), so only
    /// prefixed codes (sh/sz/bj + index prefix) are treated as indices.
    fn is_index_symbol(symbol: &Symbol) -> bool {
        if !matches!(symbol.market, Market::AShare) {
            return false;
        }
        let stripped = symbol
            .code
            .strip_prefix("sh")
            .or_else(|| symbol.code.strip_prefix("sz"))
            .or_else(|| symbol.code.strip_prefix("bj"));
        match stripped {
            Some(code) => code.starts_with("000") || code.starts_with("399") || code.starts_with("899"),
            None => false, // bare code treated as stock
        }
    }

    /// Parse a ZhituAPI timestamp string into a UTC DateTime.
    /// Tries "%Y-%m-%d %H:%M:%S" first, then falls back to "%Y-%m-%d".
    fn parse_timestamp(t: &str) -> Option<chrono::DateTime<Utc>> {
        let cst = FixedOffset::east_opt(8 * 3600).unwrap();
        NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S")
            .ok()
            .and_then(|dt| cst.from_local_datetime(&dt).single())
            .map(|t| t.with_timezone(&Utc))
            .or_else(|| {
                chrono::NaiveDate::parse_from_str(t, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .and_then(|dt| cst.from_local_datetime(&dt).single())
                    .map(|t| t.with_timezone(&Utc))
            })
    }
}

/// Response from `/hs/real/ssjy/{code}` — real-time quote.
/// The API returns numeric values (not strings), so we use f64 for decimal fields.
#[derive(Debug, Deserialize)]
pub(crate) struct ZhituQuoteResponse {
    /// Current price
    p: Option<f64>,
    /// Change amount (涨跌额)
    #[allow(dead_code)]
    ud: Option<f64>,
    /// Open
    o: Option<f64>,
    /// High
    h: Option<f64>,
    /// Low
    l: Option<f64>,
    /// Volume (成交量, 手)
    v: Option<f64>,
    /// Turnover (成交额)
    #[allow(dead_code)]
    cje: Option<f64>,
    /// PE ratio (市盈率)
    pe: Option<f64>,
    /// Market cap (总市值)
    sz: Option<f64>,
    /// Yesterday close (昨收)
    yc: Option<f64>,
    /// Timestamp
    t: Option<String>,
}

/// Response from `/hs/latest/{code}/{level}/{adjust}` or `/hs/history/...` — OHLCV bars.
/// The API returns numeric values (not strings).
#[derive(Debug, Deserialize)]
pub(crate) struct ZhituOhlcvBar {
    /// Timestamp or date string
    t: String,
    /// Open
    o: f64,
    /// High
    h: f64,
    /// Low
    l: f64,
    /// Close
    c: f64,
    /// Volume
    v: Option<f64>,
    /// Turnover (成交额)
    #[allow(dead_code)]
    a: Option<f64>,
}

fn decimal_from_f64(v: f64) -> Decimal {
    Decimal::from_f64_retain(v).unwrap_or(Decimal::ZERO)
}

/// Convert a ZhituQuoteResponse into a Quote.
pub(crate) fn parse_zhitu_quote(resp: &ZhituQuoteResponse, symbol: &Symbol) -> Result<Quote, DataError> {
    let price = resp
        .p
        .map(decimal_from_f64)
        .ok_or_else(|| DataError::Parse("missing price field 'p'".into()))?;
    let prev_close = resp
        .yc
        .map(decimal_from_f64)
        .ok_or_else(|| DataError::Parse("missing prev_close field 'yc'".into()))?;

    let change = price - prev_close;
    let change_pct = if prev_close.is_zero() {
        Decimal::ZERO
    } else {
        (change / prev_close) * Decimal::from(100)
    };

    let open = resp.o.map(decimal_from_f64);
    let high = resp.h.map(decimal_from_f64);
    let low = resp.l.map(decimal_from_f64);
    let volume = resp.v.map(|v| v as u64);
    let pe_ratio = resp.pe.map(decimal_from_f64);
    let market_cap = resp.sz.map(decimal_from_f64);

    let timestamp = resp
        .t
        .as_deref()
        .and_then(ZhituProvider::parse_timestamp)
        .unwrap_or_else(Utc::now);

    Ok(Quote {
        symbol: symbol.clone(),
        price,
        change,
        change_pct,
        open,
        high,
        low,
        volume,
        market_cap,
        pe_ratio,
        week_52_high: None,
        week_52_low: None,
        name: None,
        timestamp,
    })
}

/// Convert a list of ZhituOhlcvBar into OHLCV bars.
pub(crate) fn parse_zhitu_ohlcv(bars: &[ZhituOhlcvBar], symbol: &Symbol) -> Vec<OHLCV> {
    bars.iter()
        .filter_map(|bar| {
            let ts = ZhituProvider::parse_timestamp(&bar.t)?;
            let open = decimal_from_f64(bar.o);
            let high = decimal_from_f64(bar.h);
            let low = decimal_from_f64(bar.l);
            let close = decimal_from_f64(bar.c);
            let volume = bar.v.map(|v| v as u64).unwrap_or(0);
            Some(OHLCV {
                symbol: symbol.clone(),
                timestamp: ts,
                open,
                high,
                low,
                close,
                volume,
            })
        })
        .collect()
}

/// Calculate the start date for extended history based on period.
fn extended_start_date(period: Period) -> String {
    let now = Utc::now();
    let start = match period {
        Period::Month1 => now - chrono::Duration::days(30),
        Period::Month3 => now - chrono::Duration::days(90),
        Period::Month6 => now - chrono::Duration::days(180),
        Period::Year1 => now - chrono::Duration::days(365),
        Period::Year5 => now - chrono::Duration::days(365 * 5),
        // For intraday and other periods, use 30 days as default
        _ => now - chrono::Duration::days(30),
    };
    start.format("%Y%m%d").to_string()
}

#[async_trait]
impl DataProvider for ZhituProvider {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        if !self.supports(&symbol.market) {
            return Err(DataError::MarketNotSupported {
                market: symbol.market.to_string(),
            });
        }
        if Self::is_index_symbol(symbol) {
            return Err(DataError::MarketNotSupported {
                market: format!("ZhituAPI does not support index {}", symbol.code),
            });
        }

        let code = symbol.zhitu_bare_code();
        let url = format!(
            "{}/hs/real/ssjy/{}?token={}",
            self.base_url, code, self.token
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DataError::Network(format!("HTTP {}: {}", status, body)));
        }

        let quote_resp: ZhituQuoteResponse = resp
            .json()
            .await
            .map_err(|e| DataError::Parse(format!("JSON parse error: {}", e)))?;

        parse_zhitu_quote(&quote_resp, symbol)
    }

    async fn fetch_ohlcv(
        &self,
        symbol: &Symbol,
        period: Period,
    ) -> Result<Vec<OHLCV>, DataError> {
        if !self.supports(&symbol.market) {
            return Err(DataError::MarketNotSupported {
                market: symbol.market.to_string(),
            });
        }
        if Self::is_index_symbol(symbol) {
            return Err(DataError::MarketNotSupported {
                market: format!("ZhituAPI does not support index {}", symbol.code),
            });
        }

        let level = period.zhitu_level().ok_or_else(|| DataError::MarketNotSupported {
            market: format!("period {:?} not supported by ZhituAPI", period),
        })?;
        let adjust = zhitu_adjust();
        let code = symbol.zhitu_bare_code();
        let url = format!(
            "{}/hs/latest/{}/{}/{}?limit=500&token={}",
            self.base_url, code, level, adjust, self.token
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DataError::Network(format!("HTTP {}: {}", status, body)));
        }

        let bars: Vec<ZhituOhlcvBar> = resp
            .json()
            .await
            .map_err(|e| DataError::Parse(format!("JSON parse error: {}", e)))?;

        Ok(parse_zhitu_ohlcv(&bars, symbol))
    }

    async fn fetch_ohlcv_extended(
        &self,
        symbol: &Symbol,
        period: Period,
    ) -> Result<Vec<OHLCV>, DataError> {
        if !self.supports(&symbol.market) {
            return Err(DataError::MarketNotSupported {
                market: symbol.market.to_string(),
            });
        }
        if Self::is_index_symbol(symbol) {
            return Err(DataError::MarketNotSupported {
                market: format!("ZhituAPI does not support index {}", symbol.code),
            });
        }

        let level = period.zhitu_level().ok_or_else(|| DataError::MarketNotSupported {
            market: format!("period {:?} not supported by ZhituAPI", period),
        })?;
        let adjust = zhitu_adjust();
        let code = symbol.zhitu_bare_code();
        let st = extended_start_date(period);
        let et = Utc::now().format("%Y%m%d").to_string();
        let url = format!(
            "{}/hs/history/{}/{}/{}?st={}&et={}&token={}",
            self.base_url, code, level, adjust, st, et, self.token
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DataError::Network(format!("HTTP {}: {}", status, body)));
        }

        let bars: Vec<ZhituOhlcvBar> = resp
            .json()
            .await
            .map_err(|e| DataError::Parse(format!("JSON parse error: {}", e)))?;

        Ok(parse_zhitu_ohlcv(&bars, symbol))
    }

    fn name(&self) -> &'static str {
        "ZhituAPI"
    }

    fn supports(&self, market: &Market) -> bool {
        matches!(market, Market::AShare)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use mockito::Server;

    fn make_quote_json() -> String {
        serde_json::json!({
            "p": 1845.0,
            "ud": 15.0,
            "o": 1832.0,
            "h": 1860.0,
            "l": 1820.0,
            "v": 12345678.0,
            "cje": 22800000000.0,
            "pe": 32.5,
            "sz": 2317500000000.0,
            "yc": 1830.0,
            "t": "2025-01-15 15:00:00"
        })
        .to_string()
    }

    fn make_ohlcv_json() -> String {
        serde_json::json!([
            {
                "t": "2025-01-15",
                "o": 1832.0,
                "h": 1860.0,
                "l": 1820.0,
                "c": 1845.0,
                "v": 12345678.0,
                "a": 22800000000.0,
                "pc": 1830.0,
                "sf": 0
            },
            {
                "t": "2025-01-14",
                "o": 1828.0,
                "h": 1835.0,
                "l": 1810.0,
                "c": 1830.0,
                "v": 9876543.0,
                "a": 18000000000.0,
                "pc": 1825.0,
                "sf": 0
            }
        ])
        .to_string()
    }

    #[test]
    fn test_parse_quote_response() {
        let json_str = make_quote_json();
        let resp: ZhituQuoteResponse = serde_json::from_str(&json_str).unwrap();
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = parse_zhitu_quote(&resp, &symbol).unwrap();

        assert_eq!(quote.price, Decimal::from_f64_retain(1845.0).unwrap());
        assert_eq!(quote.change, Decimal::from_f64_retain(15.0).unwrap());
        assert_eq!(quote.open.unwrap(), Decimal::from_f64_retain(1832.0).unwrap());
        assert_eq!(quote.high.unwrap(), Decimal::from_f64_retain(1860.0).unwrap());
        assert_eq!(quote.low.unwrap(), Decimal::from_f64_retain(1820.0).unwrap());
        assert_eq!(quote.volume, Some(12345678));
        assert_eq!(quote.pe_ratio.unwrap(), Decimal::from_f64_retain(32.5).unwrap());
    }

    #[test]
    fn test_parse_ohlcv_response() {
        let json_str = make_ohlcv_json();
        let bars: Vec<ZhituOhlcvBar> = serde_json::from_str(&json_str).unwrap();
        let symbol = Symbol::new("600519", Market::AShare);
        let ohlcv = parse_zhitu_ohlcv(&bars, &symbol);

        assert_eq!(ohlcv.len(), 2);
        assert_eq!(ohlcv[0].open, Decimal::from_f64_retain(1832.0).unwrap());
        assert_eq!(ohlcv[0].high, Decimal::from_f64_retain(1860.0).unwrap());
        assert_eq!(ohlcv[0].low, Decimal::from_f64_retain(1820.0).unwrap());
        assert_eq!(ohlcv[0].close, Decimal::from_f64_retain(1845.0).unwrap());
        assert_eq!(ohlcv[0].volume, 12345678);
        assert_eq!(ohlcv[1].close, Decimal::from_f64_retain(1830.0).unwrap());
    }

    #[tokio::test]
    async fn test_fetch_quote_via_mock() {
        let mut server = Server::new_async().await;
        let json_body = make_quote_json();

        let mock = server
            .mock("GET", mockito::Matcher::Regex(r"^/hs/real/ssjy/600519".into()))
            .match_query(mockito::Matcher::UrlEncoded(
                "token".into(),
                "test_token".into(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_body)
            .create_async()
            .await;

        let provider = ZhituProvider::new("test_token").with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = provider.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, Decimal::from_f64_retain(1845.0).unwrap());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_via_mock() {
        let mut server = Server::new_async().await;
        let json_body = make_ohlcv_json();

        let mock = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/hs/latest/600519/d/fr".into()),
            )
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("limit".into(), "500".into()),
                mockito::Matcher::UrlEncoded("token".into(), "test_token".into()),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json_body)
            .create_async()
            .await;

        let provider = ZhituProvider::new("test_token").with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let bars = provider.fetch_ohlcv(&symbol, Period::Day1).await.unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].close, Decimal::from_f64_retain(1845.0).unwrap());
        mock.assert_async().await;
    }

    #[test]
    fn test_supports_only_a_share() {
        let p = ZhituProvider::new("test_token");
        assert!(p.supports(&Market::AShare));
        assert!(!p.supports(&Market::USStock));
        assert!(!p.supports(&Market::HKStock));
        assert!(!p.supports(&Market::Crypto));
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_unsupported_period() {
        let provider = ZhituProvider::new("test_token");
        let symbol = Symbol::new("600519", Market::AShare);
        let err = provider
            .fetch_ohlcv(&symbol, Period::Min1)
            .await
            .unwrap_err();
        assert!(matches!(err, DataError::MarketNotSupported { .. }));
    }

    #[test]
    fn test_is_index_symbol() {
        // Shanghai indices
        assert!(ZhituProvider::is_index_symbol(&Symbol::new(
            "sh000001",
            Market::AShare
        )));
        assert!(ZhituProvider::is_index_symbol(&Symbol::new(
            "sh000300",
            Market::AShare
        )));
        // Shenzhen indices
        assert!(ZhituProvider::is_index_symbol(&Symbol::new(
            "sz399001",
            Market::AShare
        )));
        assert!(ZhituProvider::is_index_symbol(&Symbol::new(
            "sz399006",
            Market::AShare
        )));
        // Stocks are NOT indices
        assert!(!ZhituProvider::is_index_symbol(&Symbol::new(
            "600519",
            Market::AShare
        )));
        assert!(!ZhituProvider::is_index_symbol(&Symbol::new(
            "000001",
            Market::AShare
        ))); // bare code treated as stock
        assert!(!ZhituProvider::is_index_symbol(&Symbol::new(
            "AAPL",
            Market::USStock
        )));
    }

    #[tokio::test]
    async fn test_fetch_quote_rejects_index() {
        let provider = ZhituProvider::new("test_token");
        let symbol = Symbol::new("sh000001", Market::AShare);
        let err = provider.fetch_quote(&symbol).await.unwrap_err();
        assert!(matches!(err, DataError::MarketNotSupported { .. }));
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_rejects_index() {
        let provider = ZhituProvider::new("test_token");
        let symbol = Symbol::new("sh000001", Market::AShare);
        let err = provider
            .fetch_ohlcv(&symbol, Period::Day1)
            .await
            .unwrap_err();
        assert!(matches!(err, DataError::MarketNotSupported { .. }));
    }

    #[tokio::test]
    async fn test_fetch_quote_unsupported_market() {
        let provider = ZhituProvider::new("test_token");
        let symbol = Symbol::new("AAPL", Market::USStock);
        let err = provider.fetch_quote(&symbol).await.unwrap_err();
        assert!(matches!(err, DataError::MarketNotSupported { .. }));
    }

    #[test]
    fn test_parse_timestamp_datetime() {
        let ts = ZhituProvider::parse_timestamp("2025-01-15 15:00:00").unwrap();
        // CST (UTC+8) 15:00 = UTC 07:00
        assert_eq!(ts, chrono::Utc.with_ymd_and_hms(2025, 1, 15, 7, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_timestamp_date_only() {
        let ts = ZhituProvider::parse_timestamp("2025-01-15").unwrap();
        // CST midnight = UTC previous day 16:00
        assert_eq!(ts, chrono::Utc.with_ymd_and_hms(2025, 1, 14, 16, 0, 0).unwrap());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(ZhituProvider::parse_timestamp("not-a-date").is_none());
    }
}
