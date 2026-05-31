use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use fa_core::{DataError, DataProvider, Market, Period, Quote, Symbol, OHLCV};
use rust_decimal::prelude::*;
use serde_json::Value;

pub const DEFAULT_AKTOOLS_URL: &str = "http://127.0.0.1:8080";

pub struct AkShareProvider {
    client: reqwest::Client,
    base_url: String,
}

impl AkShareProvider {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_AKTOOLS_URL.to_string())
    }

    pub fn with_base_url(url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: url.into(),
        }
    }

    pub fn minute_symbol(sym: &Symbol) -> String {
        let code = &sym.code;
        if code.starts_with("sh") || code.starts_with("sz") {
            code.clone()
        } else if code.starts_with('6') {
            format!("sh{}", code)
        } else {
            format!("sz{}", code)
        }
    }

    pub fn hist_symbol(sym: &Symbol) -> String {
        let code = &sym.code;
        if code.starts_with("sh") || code.starts_with("sz") {
            code[2..].to_string()
        } else {
            code.clone()
        }
    }
}

pub fn parse_minute_json(json: &[Value], symbol: &Symbol) -> Vec<OHLCV> {
    json.iter()
        .filter_map(|row| {
            let day_str = row["day"].as_str()?;
            let ts = NaiveDateTime::parse_from_str(day_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))?;
            let open = Decimal::from_f64(row["open"].as_f64()?)?;
            let high = Decimal::from_f64(row["high"].as_f64()?)?;
            let low = Decimal::from_f64(row["low"].as_f64()?)?;
            let close = Decimal::from_f64(row["close"].as_f64()?)?;
            let volume = row["volume"].as_f64().unwrap_or(0.0) as u64;
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

pub fn parse_hist_json(json: &[Value], symbol: &Symbol) -> Vec<OHLCV> {
    json.iter()
        .filter_map(|row| {
            let date_str = row["日期"].as_str()?;
            let ts = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .ok()
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
                .map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))?;
            let open = Decimal::from_f64(row["开盘"].as_f64()?)?;
            let high = Decimal::from_f64(row["最高"].as_f64()?)?;
            let low = Decimal::from_f64(row["最低"].as_f64()?)?;
            let close = Decimal::from_f64(row["收盘"].as_f64()?)?;
            let volume = row["成交量"].as_f64().unwrap_or(0.0) as u64;
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

#[async_trait]
impl DataProvider for AkShareProvider {
    async fn fetch_quote(&self, _symbol: &Symbol) -> Result<Quote, DataError> {
        Err(DataError::Network(
            "AkShareProvider: use Sina for real-time quotes".into(),
        ))
    }

    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError> {
        if !self.supports(&symbol.market) {
            return Err(DataError::MarketNotSupported {
                market: symbol.market.to_string(),
            });
        }

        if period.is_intraday() {
            let sym_code = Self::minute_symbol(symbol);
            let url = format!(
                "{}/api/public/stock_zh_a_minute?symbol={}&period={}&adjust=qfq",
                self.base_url,
                sym_code,
                period.akshare_period(),
            );
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| DataError::Network(e.to_string()))?
                .error_for_status()
                .map_err(|e| DataError::Network(e.to_string()))?;
            let json: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| DataError::Network(e.to_string()))?;
            Ok(parse_minute_json(&json, symbol))
        } else {
            let sym_code = Self::hist_symbol(symbol);
            let url = format!(
                "{}/api/public/stock_zh_a_hist?symbol={}&period={}&start_date=19900101&end_date=29991231&adjust=qfq",
                self.base_url,
                sym_code,
                period.akshare_period(),
            );
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| DataError::Network(e.to_string()))?
                .error_for_status()
                .map_err(|e| DataError::Network(e.to_string()))?;
            let json: Vec<Value> = resp
                .json()
                .await
                .map_err(|e| DataError::Network(e.to_string()))?;
            Ok(parse_hist_json(&json, symbol))
        }
    }

    fn name(&self) -> &'static str {
        "AkShareProvider"
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
    use rust_decimal::Decimal;
    use serde_json::json;

    fn sym() -> Symbol {
        Symbol::new("sh600519", Market::AShare)
    }

    #[test]
    fn test_minute_symbol_bare_sh() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(AkShareProvider::minute_symbol(&s), "sh600519");
    }

    #[test]
    fn test_minute_symbol_bare_sz() {
        let s = Symbol::new("000001", Market::AShare);
        assert_eq!(AkShareProvider::minute_symbol(&s), "sz000001");
    }

    #[test]
    fn test_minute_symbol_with_sh_prefix() {
        let s = Symbol::new("sh600519", Market::AShare);
        assert_eq!(AkShareProvider::minute_symbol(&s), "sh600519");
    }

    #[test]
    fn test_minute_symbol_with_sz_prefix() {
        let s = Symbol::new("sz000001", Market::AShare);
        assert_eq!(AkShareProvider::minute_symbol(&s), "sz000001");
    }

    #[test]
    fn test_hist_symbol_strips_sh_prefix() {
        let s = Symbol::new("sh600519", Market::AShare);
        assert_eq!(AkShareProvider::hist_symbol(&s), "600519");
    }

    #[test]
    fn test_hist_symbol_strips_sz_prefix() {
        let s = Symbol::new("sz000001", Market::AShare);
        assert_eq!(AkShareProvider::hist_symbol(&s), "000001");
    }

    #[test]
    fn test_hist_symbol_bare_code() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(AkShareProvider::hist_symbol(&s), "600519");
    }

    #[test]
    fn test_parse_minute_json_valid() {
        let data = vec![
            json!({"day": "2026-05-30 09:31:00", "open": 1800.0, "high": 1810.0, "low": 1795.0, "close": 1805.0, "volume": 12345}),
            json!({"day": "2026-05-30 09:32:00", "open": 1805.0, "high": 1815.0, "low": 1800.0, "close": 1812.0, "volume": 6789}),
        ];
        let bars = parse_minute_json(&data, &sym());
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].volume, 12345);
        assert_eq!(bars[1].open, Decimal::from(1805));
    }

    #[test]
    fn test_parse_minute_json_skips_invalid_date() {
        let data = vec![
            json!({"day": "not-a-date", "open": 1.0, "high": 1.0, "low": 1.0, "close": 1.0, "volume": 0}),
        ];
        assert_eq!(parse_minute_json(&data, &sym()).len(), 0);
    }

    #[test]
    fn test_parse_hist_json_valid() {
        let data = vec![
            json!({"日期": "2026-05-30", "开盘": 1800.0, "最高": 1820.0, "最低": 1790.0, "收盘": 1810.0, "成交量": 50000}),
        ];
        let bars = parse_hist_json(&data, &sym());
        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].volume, 50000);
        assert_eq!(bars[0].high, Decimal::from(1820));
    }

    #[test]
    fn test_parse_hist_json_skips_missing_fields() {
        let data = vec![json!({"日期": "2026-05-30", "开盘": 1800.0})];
        assert_eq!(parse_hist_json(&data, &sym()).len(), 0);
    }

    #[test]
    fn test_supports_ashare_only() {
        let p = AkShareProvider::new();
        assert!(p.supports(&Market::AShare));
        assert!(!p.supports(&Market::USStock));
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_minute_http_status_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .with_body("server error")
            .create_async()
            .await;

        let provider = AkShareProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let err = provider
            .fetch_ohlcv(&symbol, Period::Min1)
            .await
            .unwrap_err();

        assert!(matches!(err, DataError::Network(msg) if msg.contains("500")));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_ohlcv_hist_http_status_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .with_body("server error")
            .create_async()
            .await;

        let provider = AkShareProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let err = provider
            .fetch_ohlcv(&symbol, Period::Day1)
            .await
            .unwrap_err();

        assert!(matches!(err, DataError::Network(msg) if msg.contains("500")));
        mock.assert_async().await;
    }
}
