use crate::Market;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub code: String,
    pub market: Market,
    pub name: Option<String>,
}

impl Symbol {
    pub fn new(code: impl Into<String>, market: Market) -> Self {
        Self {
            code: code.into(),
            market,
            name: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Ticker format for Yahoo Finance API
    pub fn yahoo_ticker(&self) -> String {
        match &self.market {
            Market::AShare => {
                // Support sh/sz-prefixed codes (e.g. sh000001 for Shanghai Composite)
                if let Some(num) = self.code.strip_prefix("sh") {
                    format!("{}.SS", num)
                } else if let Some(num) = self.code.strip_prefix("sz") {
                    format!("{}.SZ", num)
                } else if self.code.starts_with('6') {
                    format!("{}.SS", self.code) // Shanghai stocks
                } else {
                    format!("{}.SZ", self.code) // Shenzhen stocks
                }
            }
            Market::HKStock => format!("{:0>4}.HK", self.code),
            _ => self.code.clone(),
        }
    }

    /// Ticker format for Sina Finance API (A股)
    pub fn sina_ticker(&self) -> String {
        match &self.market {
            Market::AShare => {
                // Pass through if already prefixed (e.g. sh000001, sz399001)
                if self.code.starts_with("sh") || self.code.starts_with("sz") {
                    return self.code.clone();
                }
                if self.code.starts_with('6') {
                    format!("sh{}", self.code)
                } else {
                    format!("sz{}", self.code)
                }
            }
            _ => self.code.clone(),
        }
    }

    /// Ticker format for ZhituAPI: `{code}.{exchange}`
    pub fn zhitu_ticker(&self) -> String {
        match &self.market {
            Market::AShare => {
                // Strip sh/sz prefix if present, then add exchange suffix
                if let Some(num) = self.code.strip_prefix("sh") {
                    format!("{}.SH", num)
                } else if let Some(num) = self.code.strip_prefix("sz") {
                    format!("{}.SZ", num)
                } else if self.code.starts_with('6') || self.code.starts_with('9') {
                    format!("{}.SH", self.code) // Shanghai
                } else if self.code.starts_with('4') || self.code.starts_with('8') {
                    format!("{}.BJ", self.code) // Beijing (NeeQ/STAR)
                } else {
                    format!("{}.SZ", self.code) // Shenzhen (0xx, 3xx)
                }
            }
            Market::HKStock => format!("{}.HK", self.code),
            _ => self.code.clone(),
        }
    }

    /// Bare stock code for ZhituAPI (no exchange suffix).
    /// Strips sh/sz prefix if present. E.g. `sh000001` → `000001`, `600519` → `600519`.
    pub fn zhitu_bare_code(&self) -> String {
        self.code
            .strip_prefix("sh")
            .or_else(|| self.code.strip_prefix("sz"))
            .unwrap_or(&self.code)
            .to_string()
    }

    /// Display code, stripping sh/sz exchange prefix if present
    pub fn display_code(&self) -> String {
        if let Some(num) = self
            .code
            .strip_prefix("sh")
            .or_else(|| self.code.strip_prefix("sz"))
        {
            num.to_string()
        } else {
            self.code.clone()
        }
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{} ({})", self.code, name)
        } else {
            write!(f, "{}", self.code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Market;

    #[test]
    fn test_yahoo_ticker_a_share() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.yahoo_ticker(), "600519.SS");
        let s2 = Symbol::new("000001", Market::AShare);
        assert_eq!(s2.yahoo_ticker(), "000001.SZ");
    }

    #[test]
    fn test_yahoo_ticker_index_with_prefix() {
        // sh prefix → Shanghai (.SS)
        let idx = Symbol::new("sh000001", Market::AShare);
        assert_eq!(idx.yahoo_ticker(), "000001.SS");
        // sz prefix → Shenzhen (.SZ)
        let idx2 = Symbol::new("sz399001", Market::AShare);
        assert_eq!(idx2.yahoo_ticker(), "399001.SZ");
    }

    #[test]
    fn test_sina_ticker_index_with_prefix() {
        let idx = Symbol::new("sh000001", Market::AShare);
        assert_eq!(idx.sina_ticker(), "sh000001");
        let idx2 = Symbol::new("sz399006", Market::AShare);
        assert_eq!(idx2.sina_ticker(), "sz399006");
    }

    #[test]
    fn test_display_code_strips_prefix() {
        let idx = Symbol::new("sh000001", Market::AShare);
        assert_eq!(idx.display_code(), "000001");
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.display_code(), "600519");
    }

    #[test]
    fn test_yahoo_ticker_us() {
        let s = Symbol::new("AAPL", Market::USStock);
        assert_eq!(s.yahoo_ticker(), "AAPL");
    }

    #[test]
    fn test_sina_ticker() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.sina_ticker(), "sh600519");
        let s2 = Symbol::new("000001", Market::AShare);
        assert_eq!(s2.sina_ticker(), "sz000001");
    }

    #[test]
    fn test_zhitu_ticker_shanghai() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.zhitu_ticker(), "600519.SH");
    }

    #[test]
    fn test_zhitu_ticker_shenzhen() {
        let s = Symbol::new("000001", Market::AShare);
        assert_eq!(s.zhitu_ticker(), "000001.SZ");
    }

    #[test]
    fn test_zhitu_ticker_index_sh_prefix() {
        let idx = Symbol::new("sh000001", Market::AShare);
        assert_eq!(idx.zhitu_ticker(), "000001.SH");
    }

    #[test]
    fn test_zhitu_ticker_index_sz_prefix() {
        let idx = Symbol::new("sz399001", Market::AShare);
        assert_eq!(idx.zhitu_ticker(), "399001.SZ");
    }

    #[test]
    fn test_zhitu_ticker_us_stock() {
        let s = Symbol::new("AAPL", Market::USStock);
        assert_eq!(s.zhitu_ticker(), "AAPL");
    }

    #[test]
    fn test_with_name() {
        let s = Symbol::new("AAPL", Market::USStock).with_name("Apple Inc.");
        assert_eq!(s.name.as_deref(), Some("Apple Inc."));
        assert_eq!(s.to_string(), "AAPL (Apple Inc.)");
    }
}
