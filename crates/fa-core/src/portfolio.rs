use crate::Symbol;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub quantity: Decimal,
    pub cost_basis: Decimal, // total cost (quantity * avg_price)
}

impl Position {
    pub fn market_value(&self, current_price: Decimal) -> Decimal {
        self.quantity * current_price
    }

    pub fn pnl(&self, current_price: Decimal) -> Decimal {
        self.market_value(current_price) - self.cost_basis
    }

    pub fn pnl_pct(&self, current_price: Decimal) -> Decimal {
        if self.cost_basis.is_zero() {
            Decimal::ZERO
        } else {
            (self.pnl(current_price) / self.cost_basis) * Decimal::from(100)
        }
    }

    pub fn cost_per_share(&self) -> Decimal {
        if self.quantity.is_zero() {
            Decimal::ZERO
        } else {
            self.cost_basis / self.quantity
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Portfolio {
    pub positions: Vec<Position>,
}

impl Portfolio {
    pub fn total_cost(&self) -> Decimal {
        self.positions.iter().map(|p| p.cost_basis).sum()
    }

    /// prices: symbol.code → current price
    pub fn total_market_value(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        self.positions
            .iter()
            .map(|p| {
                prices
                    .get(&p.symbol.code)
                    .copied()
                    .map(|price| p.market_value(price))
                    .unwrap_or(p.cost_basis)
            })
            .sum()
    }

    pub fn total_pnl(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        self.total_market_value(prices) - self.total_cost()
    }

    pub fn total_pnl_pct(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        let cost = self.total_cost();
        if cost.is_zero() {
            Decimal::ZERO
        } else {
            (self.total_pnl(prices) / cost) * Decimal::from(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Market, Symbol};
    use rust_decimal_macros::dec;

    fn aapl_position() -> Position {
        Position {
            symbol: Symbol::new("AAPL", Market::USStock),
            quantity: dec!(10),
            cost_basis: dec!(1800.00),
        }
    }

    #[test]
    fn test_market_value() {
        let pos = aapl_position();
        assert_eq!(pos.market_value(dec!(185.20)), dec!(1852.00));
    }

    #[test]
    fn test_pnl_positive() {
        let pos = aapl_position();
        assert_eq!(pos.pnl(dec!(185.20)), dec!(52.00));
    }

    #[test]
    fn test_pnl_negative() {
        let pos = aapl_position();
        assert_eq!(pos.pnl(dec!(170.00)), dec!(-100.00));
    }

    #[test]
    fn test_pnl_pct() {
        let pos = aapl_position();
        // 52/1800 * 100 = 2.888...%
        let pct = pos.pnl_pct(dec!(185.20));
        assert!(pct > dec!(2.88) && pct < dec!(2.90));
    }

    #[test]
    fn test_cost_per_share() {
        let pos = aapl_position();
        assert_eq!(pos.cost_per_share(), dec!(180.00));
    }

    #[test]
    fn test_portfolio_total_cost() {
        let p = Portfolio {
            positions: vec![
                aapl_position(),
                Position {
                    symbol: Symbol::new("TSLA", Market::USStock),
                    quantity: dec!(5),
                    cost_basis: dec!(1100.00),
                },
            ],
        };
        assert_eq!(p.total_cost(), dec!(2900.00));
    }

    #[test]
    fn test_portfolio_total_pnl() {
        let p = Portfolio {
            positions: vec![aapl_position()],
        };
        let mut prices = std::collections::HashMap::new();
        prices.insert("AAPL".to_string(), dec!(185.20));
        assert_eq!(p.total_pnl(&prices), dec!(52.00));
    }
}
