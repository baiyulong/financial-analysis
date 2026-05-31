use crate::strategy::{BarContext, Signal, Strategy};
use fa_indicator::rsi;
use rust_decimal::Decimal;

pub struct RsiStrategy {
    period: usize,
    oversold: u32,   // e.g. 30
    overbought: u32, // e.g. 70
}

impl RsiStrategy {
    pub fn new(period: usize, oversold: u32, overbought: u32) -> Self {
        assert!(period > 0, "RSI period must be positive");
        assert!(
            oversold < overbought,
            "oversold threshold must be less than overbought"
        );
        Self {
            period,
            oversold,
            overbought,
        }
    }
}

impl Strategy for RsiStrategy {
    fn name(&self) -> &str {
        "RSI 均值回归"
    }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        if history.len() <= self.period {
            return Signal::Hold;
        }
        let rsi_vals = rsi(history, self.period);
        let current = match rsi_vals.last().and_then(|v| *v) {
            Some(v) => v,
            None => return Signal::Hold,
        };
        let os = Decimal::from(self.oversold);
        let ob = Decimal::from(self.overbought);
        if current < os && ctx.position == 0 {
            Signal::BuyAll
        } else if current > ob && ctx.position > 0 {
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
    fn test_rsi_hold_when_insufficient_data() {
        let history: Vec<OHLCV> = (0..5).map(|i| bar(100.0 + i as f64)).collect();
        let mut s = RsiStrategy::new(14, 30, 70);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::Hold);
    }

    #[test]
    fn test_rsi_buys_on_oversold_series() {
        // Sharply falling series → RSI < 30 → BuyAll
        let history: Vec<OHLCV> = (0..30).map(|i| bar(200.0 - i as f64 * 5.0)).collect();
        let mut s = RsiStrategy::new(14, 30, 70);
        let signal = s.on_bar(&ctx(&history, 0));
        assert_eq!(signal, Signal::BuyAll);
    }
}
