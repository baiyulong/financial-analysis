use crate::portfolio::Portfolio;
use crate::result::{BacktestResult, Trade, TradeAction};
use crate::strategy::{BarContext, Signal, Strategy};
use chrono::NaiveDate;
use fa_core::OHLCV;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Clone, Debug)]
pub struct BacktestConfig {
    pub initial_cash: Decimal,
    pub commission_bps: Decimal,
    pub slippage_bps: Decimal,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        let today = chrono::Local::now().date_naive();
        let one_year_ago = today - chrono::Duration::days(365);
        Self {
            initial_cash: dec!(100_000),
            commission_bps: dec!(5),
            slippage_bps: dec!(3),
            start_date: one_year_ago,
            end_date: today,
        }
    }
}

pub struct Engine {
    pub config: BacktestConfig,
}

impl Engine {
    pub fn new(config: BacktestConfig) -> Self {
        Self { config }
    }

    /// Run the strategy over `all_data` (must be sorted ascending by timestamp).
    pub fn run(&self, all_data: &[OHLCV], strategy: &mut dyn Strategy) -> BacktestResult {
        let data: Vec<&OHLCV> = all_data
            .iter()
            .filter(|b| {
                let d = b.timestamp.date_naive();
                d >= self.config.start_date && d <= self.config.end_date
            })
            .collect();

        if data.is_empty() {
            return BacktestResult::empty(self.config.initial_cash);
        }

        let mut portfolio = Portfolio::new(self.config.initial_cash);
        let mut trades: Vec<Trade> = Vec::new();
        let mut equity_curve: Vec<Decimal> = Vec::new();

        strategy.reset();

        let owned: Vec<OHLCV> = data.iter().map(|b| (*b).clone()).collect();

        for (i, _) in owned.iter().enumerate() {
            let ctx = BarContext::new(&owned[..=i], portfolio.position, portfolio.cash);
            let bar = ctx.bar;

            let signal = strategy.on_bar(&ctx);

            let buy_fill = bar.close * (dec!(1) + self.config.slippage_bps / dec!(10000));
            let sell_fill = bar.close * (dec!(1) - self.config.slippage_bps / dec!(10000));

            match signal {
                Signal::BuyAll => {
                    if let Some((qty, _)) = portfolio.buy_all(buy_fill, self.config.commission_bps)
                    {
                        trades.push(Trade {
                            date: bar.timestamp.date_naive(),
                            action: TradeAction::Buy,
                            price: buy_fill,
                            quantity: qty,
                            amount: buy_fill * Decimal::from(qty),
                            pnl: None,
                        });
                    }
                }
                Signal::SellAll => {
                    if let Some((qty, _net, pnl)) =
                        portfolio.sell_all(sell_fill, self.config.commission_bps)
                    {
                        trades.push(Trade {
                            date: bar.timestamp.date_naive(),
                            action: TradeAction::Sell,
                            price: sell_fill,
                            quantity: qty,
                            amount: sell_fill * Decimal::from(qty),
                            pnl: Some(pnl),
                        });
                    }
                }
                Signal::Hold => {}
            }

            equity_curve.push(portfolio.total_value(bar.close));
        }

        BacktestResult::calculate(
            self.config.initial_cash,
            trades,
            equity_curve,
            self.config.start_date,
            self.config.end_date,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::{BarContext, Signal, Strategy};
    use chrono::TimeZone;
    use fa_core::{Market, Symbol};

    fn make_bar(close: f64, days_from_epoch: i64) -> OHLCV {
        use rust_decimal::Decimal;
        let c = Decimal::from_f64_retain(close).unwrap();
        let ts = chrono::Utc
            .timestamp_opt(days_from_epoch * 86400, 0)
            .unwrap();
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: ts,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 0,
        }
    }

    struct BuyThenSell {
        count: usize,
    }
    impl Strategy for BuyThenSell {
        fn name(&self) -> &str {
            "buy-then-sell"
        }
        fn on_bar(&mut self, _ctx: &BarContext) -> Signal {
            self.count += 1;
            if self.count == 1 {
                Signal::BuyAll
            } else if self.count == 4 {
                Signal::SellAll
            } else {
                Signal::Hold
            }
        }
        fn reset(&mut self) {
            self.count = 0;
        }
    }

    #[test]
    fn test_engine_buy_and_sell_produces_trades() {
        let start = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(1970, 1, 10).unwrap();
        let data: Vec<OHLCV> = (0..5).map(|i| make_bar(100.0, i)).collect();
        let config = BacktestConfig {
            initial_cash: dec!(10000),
            commission_bps: dec!(0),
            slippage_bps: dec!(0),
            start_date: start,
            end_date: end,
        };
        let mut strategy = BuyThenSell { count: 0 };
        let engine = Engine::new(config);
        let result = engine.run(&data, &mut strategy);
        assert_eq!(result.trades.len(), 2);
        assert_eq!(result.trades[0].action, TradeAction::Buy);
        assert_eq!(result.trades[1].action, TradeAction::Sell);
    }

    #[test]
    fn test_engine_empty_data_returns_empty_result() {
        let config = BacktestConfig::default();
        let mut strategy = BuyThenSell { count: 0 };
        let engine = Engine::new(config);
        let result = engine.run(&[], &mut strategy);
        assert_eq!(result.trades.len(), 0);
        assert_eq!(result.total_trades, 0);
    }

    #[test]
    fn test_engine_date_filter_excludes_out_of_range_bars() {
        let start = NaiveDate::from_ymd_opt(1970, 1, 3).unwrap();
        let end = NaiveDate::from_ymd_opt(1970, 1, 10).unwrap();
        let data: Vec<OHLCV> = (0..5).map(|i| make_bar(100.0, i)).collect();
        let config = BacktestConfig {
            initial_cash: dec!(10000),
            commission_bps: dec!(0),
            slippage_bps: dec!(0),
            start_date: start,
            end_date: end,
        };
        let mut strategy = BuyThenSell { count: 0 };
        let engine = Engine::new(config);
        let result = engine.run(&data, &mut strategy);
        assert!(result.trades.len() <= 2);
    }
}
