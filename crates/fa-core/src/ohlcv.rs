use crate::Symbol;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Period {
    Min1,
    Min5,
    Min15,
    Min30,
    Min60,
    Day1,
    Week1,
    Month1,
    Month3,
    Month6,
    Year1,
    Year5,
}

impl Period {
    /// Yahoo Finance range parameter
    pub fn yahoo_range(&self) -> &'static str {
        match self {
            // Minute periods are served by AkShare, not Yahoo Finance.
            // "1d" is a safe no-op fallback; callers should guard with is_intraday() first.
            Period::Min1 | Period::Min5 | Period::Min15 | Period::Min30 | Period::Min60 => "1d",
            Period::Day1 => "1d",
            Period::Week1 => "5d",
            Period::Month1 => "1mo",
            Period::Month3 => "3mo",
            Period::Month6 => "6mo",
            Period::Year1 => "1y",
            Period::Year5 => "5y",
        }
    }

    /// Yahoo Finance interval parameter
    pub fn yahoo_interval(&self) -> &'static str {
        match self {
            Period::Min1 => "1m",
            Period::Min5 => "5m",
            Period::Min15 => "15m",
            Period::Min30 => "30m",
            Period::Min60 => "60m",
            Period::Day1 => "5m",
            Period::Week1 => "1h",
            _ => "1d",
        }
    }

    /// AkShare period string for `stock_zh_a_minute` and `stock_zh_a_hist`.
    /// For periods longer than weekly, AkShare uses `"monthly"` granularity;
    /// the date-range window (not this string) controls how far back data is fetched.
    pub fn akshare_period(&self) -> &'static str {
        match self {
            Period::Min1 => "1",
            Period::Min5 => "5",
            Period::Min15 => "15",
            Period::Min30 => "30",
            Period::Min60 => "60",
            Period::Day1 => "daily",
            Period::Week1 => "weekly",
            Period::Month1 | Period::Month3 | Period::Month6 | Period::Year1 | Period::Year5 => {
                "monthly"
            }
        }
    }

    /// True for intraday (minute-level) periods.
    pub fn is_intraday(&self) -> bool {
        matches!(
            self,
            Period::Min1 | Period::Min5 | Period::Min15 | Period::Min30 | Period::Min60
        )
    }

    /// Short human-readable label for TUI display.
    pub fn label(&self) -> &'static str {
        match self {
            Period::Min1 => "1分",
            Period::Min5 => "5分",
            Period::Min15 => "15分",
            Period::Min30 => "30分",
            Period::Min60 => "60分",
            Period::Day1 => "日线",
            Period::Week1 => "周线",
            Period::Month1 => "月线",
            Period::Month3 => "季线",
            Period::Month6 => "半年",
            Period::Year1 => "年线",
            Period::Year5 => "5年",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minute_periods_akshare_label() {
        assert_eq!(Period::Min1.akshare_period(), "1");
        assert_eq!(Period::Min5.akshare_period(), "5");
        assert_eq!(Period::Min15.akshare_period(), "15");
        assert_eq!(Period::Min30.akshare_period(), "30");
        assert_eq!(Period::Min60.akshare_period(), "60");
    }

    #[test]
    fn test_is_intraday() {
        for p in [
            Period::Min1,
            Period::Min5,
            Period::Min15,
            Period::Min30,
            Period::Min60,
        ] {
            assert!(p.is_intraday(), "{p:?} should be intraday");
        }
        for p in [
            Period::Day1,
            Period::Week1,
            Period::Month1,
            Period::Month3,
            Period::Month6,
            Period::Year1,
            Period::Year5,
        ] {
            assert!(!p.is_intraday(), "{p:?} should not be intraday");
        }
    }

    #[test]
    fn test_period_label() {
        assert_eq!(Period::Min1.label(), "1分");
        assert_eq!(Period::Day1.label(), "日线");
        assert_eq!(Period::Year1.label(), "年线");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCV {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
}
