use chrono::NaiveDate;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::io::Write;

#[derive(Debug, Clone, PartialEq)]
pub enum TradeAction {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub date: NaiveDate,
    pub action: TradeAction,
    pub price: Decimal,
    pub quantity: i64,
    pub amount: Decimal,
    /// Realised PnL; Some only on Sell trades.
    pub pnl: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub total_return: Decimal,
    pub annualized_return: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub sharpe_ratio: Decimal,
    pub total_trades: usize,
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<Decimal>,
    pub initial_cash: Decimal,
    pub final_equity: Decimal,
}

impl BacktestResult {
    pub fn empty(initial_cash: Decimal) -> Self {
        Self {
            total_return: dec!(0),
            annualized_return: dec!(0),
            max_drawdown: dec!(0),
            win_rate: dec!(0),
            sharpe_ratio: dec!(0),
            total_trades: 0,
            trades: vec![],
            equity_curve: vec![],
            initial_cash,
            final_equity: initial_cash,
        }
    }

    pub fn calculate(
        initial_cash: Decimal,
        trades: Vec<Trade>,
        equity_curve: Vec<Decimal>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Self {
        let final_equity = equity_curve.last().copied().unwrap_or(initial_cash);
        let total_trades = trades.len();

        let total_return = if initial_cash > dec!(0) {
            (final_equity - initial_cash) / initial_cash
        } else {
            dec!(0)
        };

        let days = (end_date - start_date).num_days() as f64;
        let annualized_return = if days > 0.0 && initial_cash > dec!(0) {
            let tr = total_return.to_f64().unwrap_or(0.0);
            let ann = (1.0_f64 + tr).powf(365.0_f64 / days) - 1.0_f64;
            Decimal::from_f64(ann).unwrap_or(dec!(0))
        } else {
            dec!(0)
        };

        let max_drawdown = compute_max_drawdown(&equity_curve);
        let win_rate = compute_win_rate(&trades);
        let sharpe_ratio = compute_sharpe(&equity_curve);

        Self {
            total_return,
            annualized_return,
            max_drawdown,
            win_rate,
            sharpe_ratio,
            total_trades,
            trades,
            equity_curve,
            initial_cash,
            final_equity,
        }
    }

    /// Write trade log to a CSV file.
    pub fn export_csv(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::BufWriter;
        let mut f = BufWriter::new(std::fs::File::create(path)?);
        writeln!(f, "date,action,price,quantity,amount,pnl")?;
        for t in &self.trades {
            let action = match t.action {
                TradeAction::Buy => "buy",
                TradeAction::Sell => "sell",
            };
            let pnl = t.pnl.map(|p| p.to_string()).unwrap_or_default();
            writeln!(
                f,
                "{},{},{},{},{},{}",
                t.date, action, t.price, t.quantity, t.amount, pnl
            )?;
        }
        Ok(())
    }
}

fn compute_max_drawdown(equity: &[Decimal]) -> Decimal {
    if equity.is_empty() {
        return dec!(0);
    }
    let mut peak = equity[0];
    let mut max_dd = dec!(0);
    for &e in equity {
        if e > peak {
            peak = e;
        }
        if peak > dec!(0) {
            let dd = (peak - e) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    -max_dd
}

fn compute_win_rate(trades: &[Trade]) -> Decimal {
    let sells: Vec<&Trade> = trades
        .iter()
        .filter(|t| t.action == TradeAction::Sell)
        .collect();
    if sells.is_empty() {
        return dec!(0);
    }
    let wins = sells
        .iter()
        .filter(|t| t.pnl.map(|p| p > dec!(0)).unwrap_or(false))
        .count();
    Decimal::from(wins) / Decimal::from(sells.len())
}

/// Annualised Sharpe ratio using daily equity returns. Risk-free rate = 3% / 252.
fn compute_sharpe(equity: &[Decimal]) -> Decimal {
    if equity.len() < 2 {
        return dec!(0);
    }
    let returns: Vec<f64> = equity
        .windows(2)
        .map(|w| {
            let prev = w[0].to_f64().unwrap_or(1.0);
            let curr = w[1].to_f64().unwrap_or(1.0);
            if prev == 0.0 {
                0.0
            } else {
                (curr - prev) / prev
            }
        })
        .collect();
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std_dev = variance.sqrt();
    if std_dev == 0.0 {
        return dec!(0);
    }
    let rf_daily = 0.03 / 252.0;
    let sharpe = (mean - rf_daily) / std_dev * 252_f64.sqrt();
    Decimal::from_f64(sharpe).unwrap_or(dec!(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_equity(values: &[f64]) -> Vec<Decimal> {
        values
            .iter()
            .map(|v| Decimal::from_f64(*v).unwrap())
            .collect()
    }

    #[test]
    fn test_total_return_positive() {
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let equity = make_equity(&[100_000.0, 110_000.0]);
        let r = BacktestResult::calculate(dec!(100_000), vec![], equity, start, end);
        assert_eq!(r.total_return, dec!(0.1));
    }

    #[test]
    fn test_max_drawdown_correct() {
        let equity = make_equity(&[100.0, 120.0, 90.0, 110.0]);
        let dd = compute_max_drawdown(&equity);
        assert!(dd < dec!(-0.24) && dd > dec!(-0.26));
    }

    #[test]
    fn test_win_rate_half() {
        let trades = vec![
            Trade {
                date: NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
                action: TradeAction::Sell,
                price: dec!(110),
                quantity: 10,
                amount: dec!(1100),
                pnl: Some(dec!(100)),
            },
            Trade {
                date: NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
                action: TradeAction::Sell,
                price: dec!(90),
                quantity: 10,
                amount: dec!(900),
                pnl: Some(dec!(-100)),
            },
        ];
        assert_eq!(compute_win_rate(&trades), dec!(0.5));
    }

    #[test]
    fn test_export_csv_creates_file() {
        let r = BacktestResult::empty(dec!(100_000));
        let tmp = std::env::temp_dir().join("fa_backtest_test.csv");
        r.export_csv(&tmp).unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("date,action,price"));
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_empty_result() {
        let r = BacktestResult::empty(dec!(50_000));
        assert_eq!(r.total_trades, 0);
        assert_eq!(r.final_equity, dec!(50_000));
    }
}
