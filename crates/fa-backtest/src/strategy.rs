use fa_core::OHLCV;
use rust_decimal::Decimal;

/// Context passed to a strategy on each bar.
pub struct BarContext<'a> {
    /// The current bar (timestamp, OHLCV).
    pub bar: &'a OHLCV,
    /// Current shares held (0 = flat).
    pub position: i64,
    /// Available cash.
    pub cash: Decimal,
    /// All bars from the filtered start date up to and including `bar`.
    pub history: &'a [OHLCV],
}

impl<'a> BarContext<'a> {
    pub fn new(history: &'a [OHLCV], position: i64, cash: Decimal) -> Self {
        assert!(!history.is_empty(), "BarContext requires at least one bar");
        Self {
            bar: history.last().unwrap(),
            history,
            position,
            cash,
        }
    }
}

/// What the strategy wants to do after seeing a bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// Buy with all available cash at this bar's fill price.
    BuyAll,
    /// Sell entire position at this bar's fill price.
    SellAll,
    /// Do nothing.
    Hold,
}

/// Implement this to create a strategy.
pub trait Strategy: Send {
    fn name(&self) -> &str;
    fn on_bar(&mut self, ctx: &BarContext) -> Signal;
    /// Called before each new Engine::run() so the same instance can be reused.
    /// Default: no-op for stateless strategies.
    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn make_bar(close: Decimal) -> OHLCV {
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: Utc::now(),
            open: close, high: close, low: close, close, volume: 0,
        }
    }

    struct AlwaysBuy;
    impl Strategy for AlwaysBuy {
        fn name(&self) -> &str { "always-buy" }
        fn on_bar(&mut self, ctx: &BarContext) -> Signal {
            if ctx.position == 0 { Signal::BuyAll } else { Signal::Hold }
        }
    }

    #[test]
    fn test_strategy_trait_object() {
        let bar = make_bar(dec!(100));
        let history = vec![bar.clone()];
        let ctx = BarContext { bar: &bar, position: 0, cash: dec!(1000), history: &history };
        let mut s: Box<dyn Strategy> = Box::new(AlwaysBuy);
        assert_eq!(s.on_bar(&ctx), Signal::BuyAll);
    }

    #[test]
    fn test_signal_hold_when_in_position() {
        let bar = make_bar(dec!(100));
        let history = vec![bar.clone()];
        let ctx = BarContext { bar: &bar, position: 10, cash: dec!(0), history: &history };
        let mut s = AlwaysBuy;
        assert_eq!(s.on_bar(&ctx), Signal::Hold);
    }
}
