use fa_core::{DataError, Market, Portfolio, Position, Symbol};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub refresh_interval: u64,   // seconds
    pub default_currency: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 30,
            default_currency: "USD".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistEntry {
    pub symbol: String,
    pub market: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioEntry {
    pub symbol: String,
    pub market: String,
    pub quantity: String,
    pub cost_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    pub alpha_vantage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub api_keys: ApiKeys,
    #[serde(default)]
    pub watchlist: Vec<WatchlistEntry>,
    #[serde(default)]
    pub portfolio: Vec<PortfolioEntry>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("fa")
            .join("config.toml")
    }

    pub fn load() -> Result<Self, DataError> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content).map_err(|e| DataError::Config(e.to_string()))
    }

    pub fn to_watchlist_symbols(&self) -> Vec<Symbol> {
        self.watchlist.iter().filter_map(|e| {
            e.market.parse::<Market>().ok().map(|m| Symbol::new(&e.symbol, m))
        }).collect()
    }

    pub fn to_portfolio(&self) -> Portfolio {
        let positions = self.portfolio.iter().filter_map(|e| {
            let market = e.market.parse::<Market>().ok()?;
            let qty  = e.quantity.parse::<Decimal>().ok()?;
            let cost = e.cost_basis.parse::<Decimal>().ok()?;
            Some(Position {
                symbol: Symbol::new(&e.symbol, market),
                quantity: qty,
                cost_basis: cost,
            })
        }).collect();
        Portfolio { positions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.general.refresh_interval, 30);
        assert_eq!(cfg.general.default_currency, "USD");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[general]
refresh_interval = 60
default_currency = "CNY"

[[watchlist]]
symbol = "AAPL"
market = "us"

[[watchlist]]
symbol = "600519"
market = "a_share"

[[portfolio]]
symbol = "AAPL"
market = "us"
quantity = "10"
cost_basis = "1800.00"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.refresh_interval, 60);
        assert_eq!(cfg.watchlist.len(), 2);
        assert_eq!(cfg.watchlist[0].symbol, "AAPL");
        assert_eq!(cfg.portfolio.len(), 1);
    }

    #[test]
    fn test_to_watchlist_symbols() {
        let cfg = Config {
            watchlist: vec![
                WatchlistEntry { symbol: "AAPL".into(), market: "us".into() },
                WatchlistEntry { symbol: "600519".into(), market: "a_share".into() },
            ],
            ..Default::default()
        };
        let syms = cfg.to_watchlist_symbols();
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].code, "AAPL");
        assert_eq!(syms[1].market, fa_core::Market::AShare);
    }
}
