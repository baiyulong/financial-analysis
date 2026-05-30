use fa_core::{DataError, Market, Portfolio, Position, Symbol};
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn load_portfolio_from_csv(content: &str) -> Result<Portfolio, DataError> {
    let mut positions = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // skip header and blank lines
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 4 {
            return Err(DataError::Parse(format!(
                "line {}: expected 4 fields, got {}",
                i + 1,
                fields.len()
            )));
        }
        let code   = fields[0].trim().to_string();
        let market = fields[1].trim().parse::<Market>()?;
        let qty    = Decimal::from_str(fields[2].trim())
            .map_err(|_| DataError::Parse(format!("invalid quantity: {}", fields[2])))?;
        let cost   = Decimal::from_str(fields[3].trim())
            .map_err(|_| DataError::Parse(format!("invalid cost_basis: {}", fields[3])))?;

        positions.push(Position {
            symbol: Symbol::new(code, market),
            quantity: qty,
            cost_basis: cost,
        });
    }

    Ok(Portfolio { positions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_load_portfolio_from_csv() {
        let csv = include_str!("../../../fixtures/sample_portfolio.csv");
        let portfolio = load_portfolio_from_csv(csv).unwrap();
        assert_eq!(portfolio.positions.len(), 4);

        let aapl = &portfolio.positions[0];
        assert_eq!(aapl.symbol.code, "AAPL");
        assert_eq!(aapl.quantity, dec!(10));
        assert_eq!(aapl.cost_basis, dec!(1800.00));
    }

    #[test]
    fn test_load_portfolio_invalid_csv() {
        let result = load_portfolio_from_csv("symbol,market\nAAPL");
        assert!(result.is_err());
    }
}
