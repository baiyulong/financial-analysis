use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::Symbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Period {
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
            Period::Day1   => "1d",
            Period::Week1  => "5d",
            Period::Month1 => "1mo",
            Period::Month3 => "3mo",
            Period::Month6 => "6mo",
            Period::Year1  => "1y",
            Period::Year5  => "5y",
        }
    }

    /// Yahoo Finance interval parameter
    pub fn yahoo_interval(&self) -> &'static str {
        match self {
            Period::Day1  => "5m",
            Period::Week1 => "1h",
            _             => "1d",
        }
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
