use crate::strategy::{BarContext, Signal, Strategy};
use fa_indicator::sma;

pub struct MaCrossStrategy {
    fast: usize,
    slow: usize,
}

impl MaCrossStrategy {
    pub fn new(fast: usize, slow: usize) -> Self {
        assert!(fast > 0 && slow > 0, "MA periods must be positive");
        assert!(fast < slow, "fast period must be less than slow period");
        Self { fast, slow }
    }
}

impl Strategy for MaCrossStrategy {
    fn name(&self) -> &str { "双均线穿越" }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        if history.len() < self.slow + 1 {
            return Signal::Hold;
        }
        let tail = &history[history.len() - (self.slow + 1)..];
        let fast_vals = sma(tail, self.fast);
        let slow_vals = sma(tail, self.slow);
        match (fast_vals[self.slow - 1], slow_vals[self.slow - 1],
               fast_vals[self.slow],     slow_vals[self.slow]) {
            (Some(pf), Some(ps), Some(cf), Some(cs)) => {
                if pf < ps && cf > cs && ctx.position == 0 {
                    Signal::BuyAll  // golden cross: was below, now strictly above
                } else if pf > ps && cf < cs && ctx.position > 0 {
                    Signal::SellAll // death cross: was strictly above, now below
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
    fn test_genuine_golden_cross_emits_buy_all() {
        // [10, 8, 9, 15] with fast=2, slow=3:
        // prev: fast(8+9)/2=8.5, slow(10+8+9)/3=9.0 → pf < ps ✓
        // curr: fast(9+15)/2=12, slow(8+9+15)/3=10.67 → cf > cs ✓ → BuyAll
        let history = vec![bar(10.0), bar(8.0), bar(9.0), bar(15.0)];
        let mut s = MaCrossStrategy::new(2, 3);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::BuyAll);
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
