// crates/fa-data/src/sina.rs
use async_trait::async_trait;
use chrono::{FixedOffset, NaiveDate, TimeZone, Utc};
use fa_core::{DataError, DataProvider, Market, Period, Quote, Symbol, OHLCV};
use rust_decimal::prelude::*;
use serde::Deserialize;

const DEFAULT_BASE_URL: &str = "https://hq.sinajs.cn";
const KLINE_BASE_URL: &str =
    "https://money.finance.sina.com.cn/quotes_service/api/json_v2.php/CN_MarketData.getKLineData";

/// One K-line bar from Sina Finance's `getKLineData` endpoint.
/// Fields are returned as strings despite being numeric.
#[derive(Debug, Deserialize)]
struct SinaKlineBar {
    day: String,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
}

fn sina_kline_scale(period: Period) -> Option<&'static str> {
    match period {
        Period::Min1 => Some("1"),
        Period::Min5 => Some("5"),
        Period::Min15 => Some("15"),
        Period::Min30 => Some("30"),
        Period::Min60 => Some("60"),
        Period::Day1 => Some("240"),
        Period::Week1 => Some("1200"),
        Period::Month1 => Some("7200"),
        _ => None,
    }
}

fn sina_kline_datalen(period: Period) -> u32 {
    match period {
        Period::Min1 | Period::Min5 | Period::Min15 | Period::Min30 | Period::Min60 => 200,
        _ => 500,
    }
}

fn parse_sina_kline_bars(bars: &[SinaKlineBar], symbol: &Symbol) -> Vec<OHLCV> {
    let cst = FixedOffset::east_opt(8 * 3600).unwrap();
    bars.iter()
        .filter_map(|bar| {
            let naive = NaiveDate::parse_from_str(&bar.day, "%Y-%m-%d").ok()?;
            let local_dt = cst
                .from_local_datetime(&naive.and_hms_opt(0, 0, 0)?)
                .single()?;
            let timestamp = local_dt.with_timezone(&Utc);
            let open = Decimal::from_str(&bar.open).ok()?;
            let high = Decimal::from_str(&bar.high).ok()?;
            let low = Decimal::from_str(&bar.low).ok()?;
            let close = Decimal::from_str(&bar.close).ok()?;
            let volume = bar.volume.parse::<u64>().unwrap_or(0);
            Some(OHLCV {
                symbol: symbol.clone(),
                timestamp,
                open,
                high,
                low,
                close,
                volume,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct StockSuggestion {
    pub code: String,
    pub name: String,
}

pub fn parse_sina_suggest(text: &str) -> Vec<StockSuggestion> {
    let start = text.find('"').map(|i| i + 1).unwrap_or(0);
    let end = text.rfind('"').unwrap_or(text.len());
    if start >= end {
        return vec![];
    }
    let content = &text[start..end];
    if content.is_empty() {
        return vec![];
    }

    content
        .split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(',').collect();
            if parts.len() < 3 {
                return None;
            }
            let code = parts[0].trim().to_string();
            let name = parts[2].trim().to_string();
            if code.is_empty() || name.is_empty() {
                return None;
            }
            Some(StockSuggestion { code, name })
        })
        .collect()
}

pub struct SinaFinanceProvider {
    client: reqwest::Client,
    base_url: String,
    suggest_base_url: String,
    kline_base_url: String,
}

impl SinaFinanceProvider {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            suggest_base_url: "https://suggest3.sinajs.cn".to_string(),
            kline_base_url: KLINE_BASE_URL.to_string(),
        }
    }

    pub fn with_suggest_url(mut self, url: impl Into<String>) -> Self {
        self.suggest_base_url = url.into();
        self
    }

    pub fn with_kline_url(mut self, url: impl Into<String>) -> Self {
        self.kline_base_url = url.into();
        self
    }

    pub async fn search_stocks(&self, query: &str) -> Result<Vec<StockSuggestion>, DataError> {
        if query.is_empty() {
            return Ok(vec![]);
        }
        let encoded = urlencoding::encode(query);
        let url = format!(
            "{}/suggest/type=&key={}&name=&market=&rn=8",
            self.suggest_base_url, encoded
        );
        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://finance.sina.com.cn")
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;
        let text = resp
            .text()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;
        Ok(parse_sina_suggest(&text))
    }
}

/// Parse Sina Finance response text into a Quote.
/// Format: var hq_str_sh600519="name,open,prev_close,price,high,low,...";
pub fn parse_sina_response(text: &str, symbol: &Symbol) -> Result<Quote, DataError> {
    // Extract the part inside double quotes
    let start = text
        .find('"')
        .ok_or_else(|| DataError::Parse("no opening quote".into()))?
        + 1;
    let end = text
        .rfind('"')
        .ok_or_else(|| DataError::Parse("no closing quote".into()))?;

    if start >= end {
        return Err(DataError::Parse("empty response".into()));
    }

    let data = &text[start..end];
    let fields: Vec<&str> = data.split(',').collect();

    if fields.len() < 10 {
        return Err(DataError::Parse(format!(
            "too few fields: {}",
            fields.len()
        )));
    }

    let parse_dec = |s: &str| -> Result<Decimal, DataError> {
        Decimal::from_str(s.trim())
            .map_err(|_| DataError::Parse(format!("cannot parse '{}' as decimal", s)))
    };

    let name = fields[0].to_string();
    let open = parse_dec(fields[1])?;
    let prev_close = parse_dec(fields[2])?;
    let price = parse_dec(fields[3])?;
    let high = parse_dec(fields[4])?;
    let low = parse_dec(fields[5])?;
    let volume = fields[8].trim().parse::<u64>().unwrap_or(0);
    let change = price - prev_close;
    let change_pct = if prev_close.is_zero() {
        Decimal::ZERO
    } else {
        (change / prev_close) * Decimal::from(100)
    };

    Ok(Quote {
        symbol: symbol.clone(),
        price,
        change,
        change_pct,
        open: Some(open),
        high: Some(high),
        low: Some(low),
        volume: Some(volume),
        market_cap: None,
        pe_ratio: None,
        week_52_high: None,
        week_52_low: None,
        name: Some(name),
        timestamp: Utc::now(),
    })
}

#[async_trait]
impl DataProvider for SinaFinanceProvider {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        if !self.supports(&symbol.market) {
            return Err(DataError::MarketNotSupported {
                market: symbol.market.to_string(),
            });
        }

        let ticker = symbol.sina_ticker();
        let url = format!("{}/list={}", self.base_url, ticker);

        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://finance.sina.com.cn")
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(DataError::Network(format!(
                "HTTP {}",
                resp.status()
            )));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        parse_sina_response(&text, symbol)
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
        let scale = sina_kline_scale(period).ok_or_else(|| DataError::MarketNotSupported {
            market: format!("period {:?} not supported by Sina K-line", period),
        })?;
        let datalen = sina_kline_datalen(period);
        let ticker = symbol.sina_ticker();
        let url = format!(
            "{}?symbol={}&scale={}&ma=no&datalen={}",
            self.kline_base_url, ticker, scale, datalen
        );

        let resp = self
            .client
            .get(&url)
            .header("Referer", "https://finance.sina.com.cn")
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(DataError::Network(format!("HTTP {}: {}", status, body)));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;

        // Sina returns a JSON array; if the response is empty or invalid, surface it.
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == "null" {
            return Ok(vec![]);
        }

        let bars: Vec<SinaKlineBar> = serde_json::from_str(trimmed)
            .map_err(|e| DataError::Parse(format!("Sina K-line JSON error: {}", e)))?;

        Ok(parse_sina_kline_bars(&bars, symbol))
    }

    fn name(&self) -> &'static str {
        "Sina Finance"
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

    #[test]
    fn test_parse_sina_kline_bars() {
        let json = r#"[
            {"day":"2026-06-09","open":"3977.539","high":"4010.872","low":"3955.908","close":"4010.031","volume":"57657009000"},
            {"day":"2026-06-10","open":"3985.124","high":"4006.313","low":"3963.442","close":"3993.226","volume":"59869473900"}
        ]"#;
        let bars: Vec<SinaKlineBar> = serde_json::from_str(json).unwrap();
        let symbol = Symbol::new("sh000001", Market::AShare);
        let ohlcv = parse_sina_kline_bars(&bars, &symbol);
        assert_eq!(ohlcv.len(), 2);
        assert_eq!(ohlcv[0].close, Decimal::from_str("4010.031").unwrap());
        assert_eq!(ohlcv[0].volume, 57657009000);
        assert_eq!(ohlcv[1].open, Decimal::from_str("3985.124").unwrap());
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_via_mock() {
        let mut server = Server::new_async().await;
        let body = r#"[{"day":"2026-06-15","open":"4053.582","high":"4097.166","low":"4051.065","close":"4096.472","volume":"67890781100"}]"#;

        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .match_query(mockito::Matcher::Regex(r"symbol=sh000001".into()))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let provider = SinaFinanceProvider::new().with_kline_url(server.url());
        let symbol = Symbol::new("sh000001", Market::AShare);
        let bars = provider.fetch_ohlcv(&symbol, Period::Day1).await.unwrap();
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].close, Decimal::from_str("4096.472").unwrap());
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_us_stock_rejected() {
        let provider = SinaFinanceProvider::new();
        let symbol = Symbol::new("AAPL", Market::USStock);
        let err = provider
            .fetch_ohlcv(&symbol, Period::Day1)
            .await
            .unwrap_err();
        assert!(matches!(err, DataError::MarketNotSupported { .. }));
    }

    #[test]
    fn test_parse_sina_suggest_basic() {
        let text = r#"var suggestvalue="600519,11,贵州茅台,上证A股,,,,,|000001,11,平安银行,深证A股,,,,,";"#;
        let results = parse_sina_suggest(text);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].code, "600519");
        assert_eq!(results[0].name, "贵州茅台");
        assert_eq!(results[1].code, "000001");
        assert_eq!(results[1].name, "平安银行");
    }

    #[test]
    fn test_parse_sina_suggest_empty() {
        let text = r#"var suggestvalue="";"#;
        let results = parse_sina_suggest(text);
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_sina_suggest_no_quotes() {
        let results = parse_sina_suggest("no quotes here");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_parse_sina_response() {
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = parse_sina_response(fixture, &symbol).unwrap();

        assert_eq!(quote.symbol.code, "600519");
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        assert_eq!(quote.open.unwrap(), rust_decimal_macros::dec!(1832.00));
        assert_eq!(quote.high.unwrap(), rust_decimal_macros::dec!(1860.00));
        assert_eq!(quote.low.unwrap(), rust_decimal_macros::dec!(1820.00));
        assert_eq!(quote.name.as_deref(), Some("贵州茅台"));
    }

    #[tokio::test]
    async fn test_fetch_quote_sends_referer_header() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        // Mock requires Referer header — will return 501 if missing
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .match_header("Referer", "https://finance.sina.com.cn")
            .with_status(200)
            .with_body(fixture)
            .create_async()
            .await;

        let provider = SinaFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = provider.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_quote_returns_clear_error_on_403() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(403)
            .with_body("Forbidden")
            .create_async()
            .await;

        let provider = SinaFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let err = provider.fetch_quote(&symbol).await.unwrap_err();
        // Should be a Network error mentioning "403", not a confusing Parse error
        let msg = err.to_string();
        assert!(
            msg.contains("403"),
            "expected 403 in error, got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_fetch_quote_via_http() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(fixture)
            .create_async()
            .await;

        let provider = SinaFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = provider.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_quote_handles_gb18030_charset() {
        let mut server = Server::new_async().await;
        // "贵州茅台" in GBK/GB18030 bytes: b9 f3 d6 dd c3 a9 cc a8
        let gbk_name: &[u8] = &[0xb9, 0xf3, 0xd6, 0xdd, 0xc3, 0xa9, 0xcc, 0xa8];
        let ascii_suffix =
            b",1832.00,1830.00,1845.00,1860.00,1820.00,1844.90,1845.00,12345678,20250101,\";";
        let mut body: Vec<u8> = b"var hq_str_sh600519=\"".to_vec();
        body.extend_from_slice(gbk_name);
        body.extend_from_slice(ascii_suffix);

        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("Content-Type", "application/javascript; charset=GB18030")
            .with_body(body)
            .create_async()
            .await;

        let provider = SinaFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = provider.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.name.as_deref(), Some("贵州茅台"));
        mock.assert_async().await;
    }

    #[test]
    fn test_supports_only_a_share() {
        let p = SinaFinanceProvider::new();
        assert!(p.supports(&Market::AShare));
        assert!(!p.supports(&Market::USStock));
        assert!(!p.supports(&Market::Crypto));
    }
}
