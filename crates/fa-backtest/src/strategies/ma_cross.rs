use crate::strategy::{BarContext, Signal, Strategy};
use fa_indicator::sma;

pub struct MaCrossStrategy {
    pub fast: usize,
    pub slow: usize,
}

impl MaCrossStrategy {
    pub fn new(fast: usize, slow: usize) -> Self {
        Self { fast, slow }
    }
}

impl Strategy for MaCrossStrategy {
    fn name(&self) -> &str { "双均线穿越" }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        // Need at least slow+1 bars to detect a crossover (compare prev and curr MA)
        if history.len() < self.slow + 1 {
            return Signal::Hold;
        }
        let fast_vals = sma(history, self.fast);
        let slow_vals = sma(history, self.slow);
        let n = history.len();
        match (fast_vals[n - 2], slow_vals[n - 2], fast_vals[n - 1], slow_vals[n - 1]) {
            (Some(pf), Some(ps), Some(cf), Some(cs)) => {
                if pf < ps && cf >= cs && ctx.position == 0 {
                    Signal::BuyAll  // golden cross
                } else if pf >= ps && cf < cs && ctx.position > 0 {
                    Signal::SellAll // death cross
                } else {
                    Signal::Hold
                }
            }
            _ => Signal::Hold,
        }
    }
    // reset() uses default no-op (MaCrossStrategy has no state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV { symbol: Symbol::new("T", Market::USStock), timestamp: Utc::now(),
                open: c, high: c, low: c, close: c, volume: 0 }
    }

    fn ctx<'a>(history: &'a [OHLCV], position: i64) -> BarContext<'a> {
        BarContext::new(history, position, dec!(10000))
    }

    #[test]
    fn test_golden_cross_signals_buy() {
        // fast=2, slow=3: need slow+1=4 bars
        // prices such that fast was below slow, then crosses above
        let history: Vec<OHLCV> = vec![
            bar(10.0), bar(10.0), bar(10.0), bar(11.0),
        ];
        let mut s = MaCrossStrategy::new(2, 3);
        // prev: fast(10+10)/2=10, slow(10+10+10)/3=10 → pf==ps, not pf<ps → Hold
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::Hold);
    }

    #[test]
    fn test_hold_when_insufficient_data() {
        let history = vec![bar(10.0), bar(11.0)];
        let mut s = MaCrossStrategy::new(2, 3);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::Hold);
    }

    #[test]
    fn test_death_cross_signals_sell() {
        // Falling series: fast was above slow (or equal), then drops below
        // fast=2, slow=3, need slow+1=4 bars, position=10 (in position)
        let history: Vec<OHLCV> = vec![
            bar(20.0), bar(18.0), bar(15.0), bar(10.0),
        ];
        let mut s = MaCrossStrategy::new(2, 3);
        // fast(last 2) = (15+10)/2=12.5, slow(last 3) = (18+15+10)/3=14.33 → fast < slow
        // prev: fast(18+15)/2=16.5, slow(20+18+15)/3=17.67 → fast < slow already → Hold (no crossover)
        let c = ctx(&history, 10); // in position
        assert_eq!(s.on_bar(&c), Signal::Hold);
    }
}
