use fa_core::OHLCV;
use rust_decimal::Decimal;

/// 简单移动平均（SMA）。
/// - 前 period-1 个值返回 None。
/// - period == 0 或 period > data.len() 时，全部返回 None。
/// - data 应为时间升序，使用收盘价计算。
pub fn sma(data: &[OHLCV], period: usize) -> Vec<Option<Decimal>> {
    if period == 0 || data.is_empty() {
        return vec![None; data.len()];
    }
    data.iter()
        .enumerate()
        .map(|(i, _)| {
            if i + 1 < period {
                None
            } else {
                let window = &data[i + 1 - period..=i];
                let sum: Decimal = window.iter().map(|b| b.close).sum();
                Some(sum / Decimal::from(period))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn make_bar(close: Decimal) -> OHLCV {
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
        }
    }

    #[test]
    fn test_sma_period_1_equals_close() {
        let data = vec![make_bar(dec!(10)), make_bar(dec!(20)), make_bar(dec!(30))];
        let result = sma(&data, 1);
        assert_eq!(result, vec![Some(dec!(10)), Some(dec!(20)), Some(dec!(30))]);
    }

    #[test]
    fn test_sma_period_3() {
        let data = vec![
            make_bar(dec!(10)),
            make_bar(dec!(20)),
            make_bar(dec!(30)),
            make_bar(dec!(40)),
        ];
        let result = sma(&data, 3);
        assert_eq!(result[0], None);
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some(dec!(20)));
        assert_eq!(result[3], Some(dec!(30)));
    }

    #[test]
    fn test_sma_period_greater_than_data_returns_all_none() {
        let data = vec![make_bar(dec!(100)), make_bar(dec!(200))];
        let result = sma(&data, 5);
        assert_eq!(result, vec![None, None]);
    }

    #[test]
    fn test_sma_empty_data() {
        let result = sma(&[], 5);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_sma_period_zero_returns_all_none() {
        let data = vec![make_bar(dec!(10)), make_bar(dec!(20))];
        let result = sma(&data, 0);
        assert_eq!(result, vec![None, None]);
    }
}
