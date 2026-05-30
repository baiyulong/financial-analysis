use serde::{Deserialize, Serialize};
use crate::Market;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub code: String,
    pub market: Market,
    pub name: Option<String>,
}

impl Symbol {
    pub fn new(code: impl Into<String>, market: Market) -> Self {
        Self { code: code.into(), market, name: None }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Ticker format for Yahoo Finance API
    pub fn yahoo_ticker(&self) -> String {
        match &self.market {
            Market::AShare => {
                if self.code.starts_with('6') {
                    format!("{}.SS", self.code) // Shanghai
                } else {
                    format!("{}.SZ", self.code) // Shenzhen
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
                if self.code.starts_with('6') {
                    format!("sh{}", self.code)
                } else {
                    format!("sz{}", self.code)
                }
            }
            _ => self.code.clone(),
        }
    }

    pub fn display_code(&self) -> String {
        self.code.clone()
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
    fn test_with_name() {
        let s = Symbol::new("AAPL", Market::USStock).with_name("Apple Inc.");
        assert_eq!(s.name.as_deref(), Some("Apple Inc."));
        assert_eq!(s.to_string(), "AAPL (Apple Inc.)");
    }
}
