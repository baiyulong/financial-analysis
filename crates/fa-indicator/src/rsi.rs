use fa_core::OHLCV;
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};

/// RSI using Wilder's smoothing method.
/// Returns Vec of same length as `data`.
/// First `period` elements are None; element at index `period` is the first RSI value.
pub fn rsi(data: &[OHLCV], period: usize) -> Vec<Option<Decimal>> {
    if period == 0 || data.len() <= period {
        return vec![None; data.len()];
    }

    let mut result = vec![None; data.len()];
    let closes: Vec<f64> = data.iter().map(|b| b.close.to_f64().unwrap_or(0.0)).collect();

    // Seed: average gain/loss over first `period` changes
    let mut avg_gain: f64 = 0.0;
    let mut avg_loss: f64 = 0.0;
    for i in 1..=period {
        let change = closes[i] - closes[i - 1];
        if change > 0.0 { avg_gain += change; } else { avg_loss += change.abs(); }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    result[period] = Some(rsi_from_avg(avg_gain, avg_loss));

    // Wilder smoothing
    for i in (period + 1)..closes.len() {
        let change = closes[i] - closes[i - 1];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { change.abs() } else { 0.0 };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        result[i] = Some(rsi_from_avg(avg_gain, avg_loss));
    }

    result
}

fn rsi_from_avg(avg_gain: f64, avg_loss: f64) -> Decimal {
    let val = if avg_loss == 0.0 {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
    };
    Decimal::from_f64(val).unwrap_or_else(|| Decimal::from_f64(50.0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV { symbol: Symbol::new("T", Market::USStock), timestamp: Utc::now(),
                open: c, high: c, low: c, close: c, volume: 0 }
    }

    #[test]
    fn test_rsi_length_matches_input() {
        let data: Vec<OHLCV> = (0..20).map(|i| bar(100.0 + i as f64)).collect();
        let result = rsi(&data, 14);
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_rsi_first_period_elements_are_none() {
        let data: Vec<OHLCV> = (0..20).map(|i| bar(100.0 + i as f64)).collect();
        let result = rsi(&data, 14);
        for i in 0..14 { assert!(result[i].is_none(), "index {i} should be None"); }
    }

    #[test]
    fn test_rsi_all_up_approaches_100() {
        let data: Vec<OHLCV> = (0..30).map(|i| bar(100.0 + i as f64 * 2.0)).collect();
        let result = rsi(&data, 14);
        let last = result.last().unwrap().unwrap();
        assert!(last > dec!(90), "expected RSI > 90 for all-up series, got {last}");
    }

    #[test]
    fn test_rsi_all_down_approaches_0() {
        let data: Vec<OHLCV> = (0..30).map(|i| bar(200.0 - i as f64 * 2.0)).collect();
        let result = rsi(&data, 14);
        let last = result.last().unwrap().unwrap();
        assert!(last < dec!(10), "expected RSI < 10 for all-down series, got {last}");
    }

    #[test]
    fn test_rsi_too_short_returns_all_none() {
        let data: Vec<OHLCV> = (0..5).map(|i| bar(100.0 + i as f64)).collect();
        let result = rsi(&data, 14);
        assert!(result.iter().all(|r| r.is_none()));
    }
}
