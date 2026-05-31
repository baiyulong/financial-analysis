use fa_core::{DataError, Market, Portfolio, Position, Symbol};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub refresh_interval: u64, // seconds
    pub default_currency: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 30,
            default_currency: "CNY".to_string(),
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
pub struct DataSourceConfig {
    pub provider: Option<String>,
    pub akshare_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub api_keys: ApiKeys,
    #[serde(default)]
    pub data_source: DataSourceConfig,
    #[serde(default)]
    pub watchlist: Vec<WatchlistEntry>,
    #[serde(default)]
    pub portfolio: Vec<PortfolioEntry>,
}

impl Config {
    /// Load the embedded default configuration (config/default.toml bundled at compile time).
    pub fn load_defaults() -> Self {
        let default_toml = include_str!("../config/default.toml");
        toml::from_str(default_toml).unwrap_or_default()
    }

    pub fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home = dirs::home_dir().ok_or("cannot find home dir")?;
        Ok(home.join(".config").join("fa").join("config.toml"))
    }

    pub fn load() -> Result<Self, DataError> {
        let path = Self::config_path().map_err(|e| DataError::Config(e.to_string()))?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content).map_err(|e| DataError::Config(e.to_string()))
    }

    pub fn save_data_source(
        provider: &str,
        akshare_url: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        let existing = if config_path.exists() {
            std::fs::read_to_string(&config_path)?
        } else {
            String::new()
        };
        let without_section = remove_toml_section(&existing, "data_source");
        let prefix = without_section.trim_end();
        let section_body = toml::to_string(&DataSourceConfig {
            provider: Some(provider.to_string()),
            akshare_url: Some(akshare_url.to_string()),
        })?;
        let new_content = if prefix.is_empty() {
            format!("[data_source]\n{}", section_body)
        } else {
            format!("{}\n\n[data_source]\n{}", prefix, section_body)
        };
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&config_path, new_content)?;
        Ok(())
    }

    pub fn to_watchlist_symbols(&self) -> Vec<Symbol> {
        self.watchlist
            .iter()
            .filter_map(|e| {
                let m = e.market.parse::<Market>().ok()?;
                Some(Symbol::new(&e.symbol, m))
            })
            .collect()
    }

    pub fn to_portfolio(&self) -> Portfolio {
        let positions = self
            .portfolio
            .iter()
            .filter_map(|e| {
                let market = match e.market.parse::<Market>() {
                    Ok(m) => m,
                    Err(_) => {
                        eprintln!(
                            "Config: skipping portfolio entry '{}': invalid market '{}'",
                            e.symbol, e.market
                        );
                        return None;
                    }
                };
                let qty = match Decimal::from_str(&e.quantity) {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!(
                            "Config: skipping portfolio entry '{}': invalid quantity '{}'",
                            e.symbol, e.quantity
                        );
                        return None;
                    }
                };
                let cost = match Decimal::from_str(&e.cost_basis) {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!(
                            "Config: skipping portfolio entry '{}': invalid cost_basis '{}'",
                            e.symbol, e.cost_basis
                        );
                        return None;
                    }
                };
                Some(Position {
                    symbol: Symbol::new(&e.symbol, market),
                    quantity: qty,
                    cost_basis: cost,
                })
            })
            .collect();
        Portfolio { positions }
    }
}

fn remove_toml_section(content: &str, section: &str) -> String {
    let header = format!("[{}]", section);
    let mut result = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        if line.trim_start().starts_with('[') {
            in_section = line.trim() == header;
        }
        if !in_section {
            result.push(line);
        }
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    fn home_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn test_home_dir(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-homes")
            .join(name)
    }

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.general.refresh_interval, 30);
        assert_eq!(cfg.general.default_currency, "CNY");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[general]
refresh_interval = 60
default_currency = "CNY"

[data_source]
provider = "akshare"
akshare_url = "http://127.0.0.1:9000"

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
        assert_eq!(cfg.data_source.provider.as_deref(), Some("akshare"));
        assert_eq!(
            cfg.data_source.akshare_url.as_deref(),
            Some("http://127.0.0.1:9000")
        );
        assert_eq!(cfg.watchlist.len(), 2);
        assert_eq!(cfg.watchlist[0].symbol, "AAPL");
        assert_eq!(cfg.portfolio.len(), 1);
    }

    #[test]
    fn test_remove_toml_section_drops_only_target_section() {
        let content = r#"
[general]
refresh_interval = 30

[data_source]
provider = "sina"
akshare_url = "http://old"

[[watchlist]]
symbol = "AAPL"
market = "us"
"#;

        let updated = remove_toml_section(content, "data_source");

        assert!(updated.contains("[general]"));
        assert!(updated.contains("[[watchlist]]"));
        assert!(!updated.contains("[data_source]"));
        assert!(!updated.contains("provider = \"sina\""));
    }

    #[test]
    fn test_save_data_source_replaces_existing_section() {
        let _guard = home_lock().lock().unwrap();
        let test_home = test_home_dir("config-save-data-source");
        if test_home.exists() {
            std::fs::remove_dir_all(&test_home).unwrap();
        }
        let config_path = test_home.join(".config").join("fa").join("config.toml");
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::write(
            &config_path,
            "[general]\nrefresh_interval = 30\n\n[data_source]\nprovider = \"sina\"\nakshare_url = \"http://old\"\n",
        )
        .unwrap();

        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &test_home);

        Config::save_data_source("akshare", "http://127.0.0.1:9000").unwrap();

        match original_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }

        let saved = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(saved.matches("[data_source]").count(), 1);
        assert!(saved.contains("provider = \"akshare\""));
        assert!(saved.contains("akshare_url = \"http://127.0.0.1:9000\""));
        assert!(saved.contains("[general]"));

        std::fs::remove_dir_all(&test_home).unwrap();
    }

    #[test]
    fn test_save_data_source_escapes_toml_values() {
        let _guard = home_lock().lock().unwrap();
        let test_home = test_home_dir("config-save-data-source-escaped");
        if test_home.exists() {
            std::fs::remove_dir_all(&test_home).unwrap();
        }

        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &test_home);

        let url = "http://127.0.0.1:9000?token=\"quoted\"";
        Config::save_data_source("akshare", url).unwrap();

        match original_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }

        let config_path = test_home.join(".config").join("fa").join("config.toml");
        let saved = std::fs::read_to_string(&config_path).unwrap();
        let parsed: Config = toml::from_str(&saved).unwrap();
        assert_eq!(parsed.data_source.provider.as_deref(), Some("akshare"));
        assert_eq!(parsed.data_source.akshare_url.as_deref(), Some(url));

        std::fs::remove_dir_all(&test_home).unwrap();
    }

    #[test]
    fn test_load_missing_config_does_not_create_config_dir() {
        let _guard = home_lock().lock().unwrap();
        let test_home = test_home_dir("config-load-missing");
        if test_home.exists() {
            std::fs::remove_dir_all(&test_home).unwrap();
        }

        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", &test_home);

        let cfg = Config::load().unwrap();

        match original_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }

        assert_eq!(cfg.general.refresh_interval, 30);
        assert!(!test_home.join(".config").join("fa").exists());
    }

    #[test]
    fn test_to_watchlist_symbols() {
        let cfg = Config {
            watchlist: vec![
                WatchlistEntry {
                    symbol: "AAPL".into(),
                    market: "us".into(),
                },
                WatchlistEntry {
                    symbol: "600519".into(),
                    market: "a_share".into(),
                },
            ],
            ..Default::default()
        };
        let syms = cfg.to_watchlist_symbols();
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].code, "AAPL");
        assert_eq!(syms[1].market, fa_core::Market::AShare);
    }
}
