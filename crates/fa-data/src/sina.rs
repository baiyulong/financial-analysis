// crates/fa-data/src/sina.rs
use async_trait::async_trait;
use chrono::Utc;
use fa_core::{DataError, DataProvider, Market, Quote, Symbol, OHLCV, Period};
use rust_decimal::prelude::*;

const DEFAULT_BASE_URL: &str = "https://hq.sinajs.cn";

#[derive(Debug, Clone)]
pub struct StockSuggestion {
    pub code: String,
    pub name: String,
}

pub fn parse_sina_suggest(text: &str) -> Vec<StockSuggestion> {
    let start = text.find('"').map(|i| i + 1).unwrap_or(0);
    let end   = text.rfind('"').unwrap_or(text.len());
    if start >= end { return vec![]; }
    let content = &text[start..end];
    if content.is_empty() { return vec![]; }

    content.split('|').filter_map(|entry| {
        let parts: Vec<&str> = entry.split(',').collect();
        if parts.len() < 3 { return None; }
        let code = parts[0].trim().to_string();
        let name = parts[2].trim().to_string();
        if code.is_empty() || name.is_empty() { return None; }
        Some(StockSuggestion { code, name })
    }).collect()
}

pub struct SinaFinanceProvider {
    client: reqwest::Client,
    base_url: String,
    suggest_base_url: String,
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
        }
    }

    pub fn with_suggest_url(mut self, url: impl Into<String>) -> Self {
        self.suggest_base_url = url.into();
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
        let resp = self.client
            .get(&url)
            .header("Referer", "https://finance.sina.com.cn")
            .send()
            .await
            .map_err(|e| DataError::Network(e.to_string()))?;
        let text = resp.text().await
            .map_err(|e| DataError::Network(e.to_string()))?;
        Ok(parse_sina_suggest(&text))
    }
}

/// Parse Sina Finance response text into a Quote.
/// Format: var hq_str_sh600519="name,open,prev_close,price,high,low,...";
pub fn parse_sina_response(text: &str, symbol: &Symbol) -> Result<Quote, DataError> {
    // Extract the part inside double quotes
    let start = text.find('"').ok_or_else(|| DataError::Parse("no opening quote".into()))? + 1;
    let end   = text.rfind('"').ok_or_else(|| DataError::Parse("no closing quote".into()))?;
    
    if start >= end {
        return Err(DataError::Parse("empty response".into()));
    }
    
    let data  = &text[start..end];
    let fields: Vec<&str> = data.split(',').collect();
    
    if fields.len() < 10 {
        return Err(DataError::Parse(format!("too few fields: {}", fields.len())));
    }

    let parse_dec = |s: &str| -> Result<Decimal, DataError> {
        Decimal::from_str(s.trim()).map_err(|_| DataError::Parse(format!("cannot parse '{}' as decimal", s)))
    };

    let name       = fields[0].to_string();
    let open       = parse_dec(fields[1])?;
    let prev_close = parse_dec(fields[2])?;
    let price      = parse_dec(fields[3])?;
    let high       = parse_dec(fields[4])?;
    let low        = parse_dec(fields[5])?;
    let volume     = fields[8].trim().parse::<u64>().unwrap_or(0);
    let change     = price - prev_close;
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
            return Err(DataError::MarketNotSupported { market: symbol.market.to_string() });
        }

        let ticker = symbol.sina_ticker();
        let url = format!("{}/list={}", self.base_url, ticker);

        let resp = self.client.get(&url).send().await
            .map_err(|e| DataError::Network(e.to_string()))?;
        let text = resp.text().await
            .map_err(|e| DataError::Network(e.to_string()))?;

        parse_sina_response(&text, symbol)
    }

    async fn fetch_ohlcv(&self, _symbol: &Symbol, _period: Period) -> Result<Vec<OHLCV>, DataError> {
        // Sina does not provide OHLCV history; use Yahoo for history
        Err(DataError::MarketNotSupported {
            market: "OHLCV not supported by Sina provider".into(),
        })
    }

    fn name(&self) -> &'static str { "Sina Finance" }

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

    #[test]
    fn test_supports_only_a_share() {
        let p = SinaFinanceProvider::new();
        assert!(p.supports(&Market::AShare));
        assert!(!p.supports(&Market::USStock));
        assert!(!p.supports(&Market::Crypto));
    }
}
