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

    /// Yahoo Finance range string for "load more history" requests.
    /// Extends the lookback window while keeping the same `yahoo_interval()`.
    pub fn extended_yahoo_range(&self) -> &'static str {
        match self {
            Period::Month1 | Period::Month3 | Period::Month6 | Period::Year1 => "5y",
            Period::Year5 => "max",
            // Intraday and sub-daily periods: Yahoo doesn't provide deep history
            _ => self.yahoo_range(),
        }
    }

    /// True if fetching more historical data is meaningful for this period.
    /// Daily+ charts (Month1…Year5) support extended range.
    /// Intraday and Day1/Week1 charts have limited history in Yahoo Finance.
    pub fn can_extend_history(&self) -> bool {
        !self.is_intraday() && !matches!(self, Period::Day1 | Period::Week1)
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

    /// ZhituAPI level parameter for OHLCV endpoints.
    /// Returns None for periods not supported by ZhituAPI.
    pub fn zhitu_level(&self) -> Option<&'static str> {
        match self {
            Period::Min5 => Some("5"),
            Period::Min15 => Some("15"),
            Period::Min30 => Some("30"),
            Period::Min60 => Some("60"),
            Period::Day1 => Some("d"),
            Period::Week1 => Some("w"),
            Period::Month1 => Some("m"),
            Period::Year1 => Some("y"),
            _ => None, // Min1, Month3, Month6, Year5 not supported
        }
    }
}

/// ZhituAPI adjust type. Default: forward-adjusted ("qfq" style = "fr").
pub fn zhitu_adjust() -> &'static str {
    "fr" // forward-adjusted (前复权)
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
    fn test_can_extend_history() {
        for p in [Period::Min1, Period::Min5, Period::Day1, Period::Week1] {
            assert!(!p.can_extend_history(), "{p:?} should not support extended history");
        }
        for p in [
            Period::Month1,
            Period::Month3,
            Period::Month6,
            Period::Year1,
            Period::Year5,
        ] {
            assert!(p.can_extend_history(), "{p:?} should support extended history");
        }
    }

    #[test]
    fn test_extended_yahoo_range() {
        assert_eq!(Period::Month1.extended_yahoo_range(), "5y");
        assert_eq!(Period::Month3.extended_yahoo_range(), "5y");
        assert_eq!(Period::Month6.extended_yahoo_range(), "5y");
        assert_eq!(Period::Year1.extended_yahoo_range(), "5y");
        assert_eq!(Period::Year5.extended_yahoo_range(), "max");
        // Non-extendable periods return their default range
        assert_eq!(Period::Day1.extended_yahoo_range(), Period::Day1.yahoo_range());
        assert_eq!(Period::Min1.extended_yahoo_range(), Period::Min1.yahoo_range());
    }

    #[test]
    fn test_period_label() {
        assert_eq!(Period::Min1.label(), "1分");
        assert_eq!(Period::Day1.label(), "日线");
        assert_eq!(Period::Year1.label(), "年线");
    }

    #[test]
    fn test_zhitu_level_supported() {
        assert_eq!(Period::Min5.zhitu_level(), Some("5"));
        assert_eq!(Period::Min15.zhitu_level(), Some("15"));
        assert_eq!(Period::Min30.zhitu_level(), Some("30"));
        assert_eq!(Period::Min60.zhitu_level(), Some("60"));
        assert_eq!(Period::Day1.zhitu_level(), Some("d"));
        assert_eq!(Period::Week1.zhitu_level(), Some("w"));
        assert_eq!(Period::Month1.zhitu_level(), Some("m"));
        assert_eq!(Period::Year1.zhitu_level(), Some("y"));
    }

    #[test]
    fn test_zhitu_level_unsupported() {
        assert_eq!(Period::Min1.zhitu_level(), None);
        assert_eq!(Period::Month3.zhitu_level(), None);
        assert_eq!(Period::Month6.zhitu_level(), None);
        assert_eq!(Period::Year5.zhitu_level(), None);
    }

    #[test]
    fn test_zhitu_adjust() {
        assert_eq!(zhitu_adjust(), "fr");
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
