use crate::strategy::{BarContext, Signal, Strategy};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

pub struct BollingerStrategy {
    period: usize,
    std_dev: f64,
}

impl BollingerStrategy {
    pub fn new(period: usize, std_dev: f64) -> Self {
        assert!(period >= 2, "Bollinger period must be at least 2");
        assert!(std_dev > 0.0, "std_dev multiplier must be positive");
        Self { period, std_dev }
    }
}

impl Strategy for BollingerStrategy {
    fn name(&self) -> &str {
        "布林带"
    }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        if history.len() < self.period {
            return Signal::Hold;
        }
        if self.period < 2 {
            return Signal::Hold;
        }

        let window = &history[history.len() - self.period..];
        let closes_f64: Vec<f64> = window
            .iter()
            .map(|b| b.close.to_f64().unwrap_or(0.0))
            .collect();
        let mean = closes_f64.iter().sum::<f64>() / self.period as f64;
        let variance =
            closes_f64.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / (self.period - 1) as f64;
        let std = variance.sqrt();

        let upper = Decimal::from_f64(mean + self.std_dev * std).unwrap_or(Decimal::MAX);
        let lower = Decimal::from_f64(mean - self.std_dev * std).unwrap_or(Decimal::MIN);

        let current_close = ctx.bar.close;

        if current_close <= lower && ctx.position == 0 {
            Signal::BuyAll
        } else if current_close >= upper && ctx.position > 0 {
            Signal::SellAll
        } else {
            Signal::Hold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV {
            symbol: Symbol::new("T", Market::USStock),
            timestamp: Utc::now(),
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 0,
        }
    }

    fn ctx<'a>(history: &'a [OHLCV], position: i64) -> BarContext<'a> {
        BarContext::new(history, position, dec!(10000))
    }

    #[test]
    fn test_bollinger_hold_when_insufficient_data() {
        let history: Vec<OHLCV> = (0..5).map(|_| bar(100.0)).collect();
        let mut s = BollingerStrategy::new(20, 2.0);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::Hold);
    }

    #[test]
    fn test_bollinger_buys_below_lower_band() {
        // Stable prices then a big drop — last price falls below lower band
        let mut history: Vec<OHLCV> = (0..19).map(|_| bar(100.0)).collect();
        history.push(bar(80.0)); // sharp drop — should be below lower band (mean≈99, std≈1, lower≈97)
        let mut s = BollingerStrategy::new(20, 2.0);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::BuyAll);
    }

    #[test]
    fn test_bollinger_sells_above_upper_band() {
        // Stable prices then a big spike — last price above upper band
        let mut history: Vec<OHLCV> = (0..19).map(|_| bar(100.0)).collect();
        history.push(bar(120.0)); // spike — should be above upper band
        let mut s = BollingerStrategy::new(20, 2.0);
        assert_eq!(s.on_bar(&ctx(&history, 10)), Signal::SellAll); // in position
    }
}
