use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Internal position/cash tracker. Not part of the public API.
pub(crate) struct Portfolio {
    pub cash: Decimal,
    pub position: i64,
    /// All-in cost of shares held (including buy commission).
    pub cost_basis: Decimal,
}

impl Portfolio {
    pub fn new(initial_cash: Decimal) -> Self {
        Self {
            cash: initial_cash,
            position: 0,
            cost_basis: dec!(0),
        }
    }

    pub fn total_value(&self, current_price: Decimal) -> Decimal {
        self.cash + current_price * Decimal::from(self.position)
    }

    /// Buy as many shares as cash allows at `fill_price`.
    /// Returns (shares_bought, cost_per_share_excl_commission) or None if cannot buy.
    pub fn buy_all(
        &mut self,
        fill_price: Decimal,
        commission_bps: Decimal,
    ) -> Option<(i64, Decimal)> {
        if self.cash <= dec!(0) || self.position > 0 || fill_price <= dec!(0) {
            return None;
        }
        let commission_rate = commission_bps / dec!(10000);
        let shares_dec = (self.cash / (fill_price * (dec!(1) + commission_rate))).floor();
        let shares = shares_dec.to_i64()?;
        if shares <= 0 {
            return None;
        }
        let cost = fill_price * Decimal::from(shares);
        let commission = cost * commission_rate;
        self.cash -= cost + commission;
        self.position += shares;
        self.cost_basis += cost + commission;
        Some((shares, fill_price))
    }

    /// Sell entire position at `fill_price`.
    /// Returns (shares_sold, net_proceeds, pnl) or None if no position.
    pub fn sell_all(
        &mut self,
        fill_price: Decimal,
        commission_bps: Decimal,
    ) -> Option<(i64, Decimal, Decimal)> {
        if self.position <= 0 {
            return None;
        }
        let shares = self.position;
        let gross = fill_price * Decimal::from(shares);
        let commission = gross * commission_bps / dec!(10000);
        let net = gross - commission;
        let pnl = net - self.cost_basis;
        self.cash += net;
        self.position = 0;
        self.cost_basis = dec!(0);
        Some((shares, net, pnl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buy_all_consumes_cash() {
        let mut p = Portfolio::new(dec!(10000));
        // fill_price=100, commission=5bps=0.05% → shares = floor(10000 / (100 * 1.0005)) = 99
        let result = p.buy_all(dec!(100), dec!(5));
        assert!(result.is_some());
        let (shares, _) = result.unwrap();
        assert_eq!(shares, 99);
        assert!(p.cash >= dec!(0));
        assert_eq!(p.position, 99);
    }

    #[test]
    fn test_sell_all_clears_position() {
        let mut p = Portfolio::new(dec!(10000));
        p.buy_all(dec!(100), dec!(5));
        let result = p.sell_all(dec!(110), dec!(5));
        assert!(result.is_some());
        let (shares, net, pnl) = result.unwrap();
        assert_eq!(shares, 99);
        assert!(net > dec!(0));
        assert!(pnl > dec!(0));
        assert_eq!(p.position, 0);
    }

    #[test]
    fn test_cannot_buy_when_in_position() {
        let mut p = Portfolio::new(dec!(10000));
        p.buy_all(dec!(100), dec!(5));
        assert!(p.buy_all(dec!(100), dec!(5)).is_none());
    }

    #[test]
    fn test_cannot_sell_when_flat() {
        let mut p = Portfolio::new(dec!(10000));
        assert!(p.sell_all(dec!(100), dec!(5)).is_none());
    }

    #[test]
    fn test_total_value() {
        let mut p = Portfolio::new(dec!(10000));
        p.buy_all(dec!(100), dec!(0));
        let val = p.total_value(dec!(120));
        assert!(val > dec!(10000));
    }
}
