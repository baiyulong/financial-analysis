use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::Symbol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: Symbol,
    pub price: Decimal,
    pub change: Decimal,
    pub change_pct: Decimal,
    pub open: Option<Decimal>,
    pub high: Option<Decimal>,
    pub low: Option<Decimal>,
    pub volume: Option<u64>,
    pub market_cap: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
    pub week_52_high: Option<Decimal>,
    pub week_52_low: Option<Decimal>,
    pub name: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Quote {
    pub fn is_positive(&self) -> bool {
        self.change >= Decimal::ZERO
    }

    pub fn change_sign(&self) -> &'static str {
        if self.change >= Decimal::ZERO { "+" } else { "" }
    }

    /// Format: "+1.25 (+0.68%)"
    pub fn change_display(&self) -> String {
        format!(
            "{}{:.2} ({}{:.2}%)",
            self.change_sign(), self.change,
            self.change_sign(), self.change_pct
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Market, Symbol};
    use rust_decimal_macros::dec;

    fn make_quote(price: Decimal, change: Decimal, change_pct: Decimal) -> Quote {
        Quote {
            symbol: Symbol::new("AAPL", Market::USStock),
            price,
            change,
            change_pct,
            open: None,
            high: None,
            low: None,
            volume: None,
            market_cap: None,
            pe_ratio: None,
            week_52_high: None,
            week_52_low: None,
            name: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_positive_change() {
        let q = make_quote(dec!(185.20), dec!(2.20), dec!(1.20));
        assert!(q.is_positive());
        assert_eq!(q.change_sign(), "+");
        assert_eq!(q.change_display(), "+2.20 (+1.20%)");
    }

    #[test]
    fn test_negative_change() {
        let q = make_quote(dec!(245.80), dec!(-2.10), dec!(-0.85));
        assert!(!q.is_positive());
        assert_eq!(q.change_sign(), "");
        assert_eq!(q.change_display(), "-2.10 (-0.85%)");
    }
}
