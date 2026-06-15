/// SQLite-based persistent storage for fa.
///
/// Database file: `./fa.db` (current working directory).
/// Schema migrations run automatically on open.
use anyhow::Result;
use fa_core::{Market, Portfolio, Position, Symbol};
use rusqlite::{params, Connection};
use rust_decimal::Decimal;
use std::path::Path;
use std::str::FromStr;

/// Stable DB key for a Market (roundtrips through FromStr).
fn market_key(m: &Market) -> &'static str {
    match m {
        Market::AShare => "a_share",
        Market::USStock => "us",
        Market::HKStock => "hk",
        Market::Crypto => "crypto",
        Market::Forex => "forex",
    }
}

pub struct Storage {
    conn: Connection,
}

impl Storage {
    /// Open (or create) the SQLite database at `path` and run migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let s = Self { conn };
        s.migrate()?;
        Ok(s)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS watchlist (
                symbol   TEXT NOT NULL,
                market   TEXT NOT NULL,
                position INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (symbol, market)
            );
            CREATE TABLE IF NOT EXISTS portfolio (
                symbol     TEXT NOT NULL,
                market     TEXT NOT NULL,
                quantity   TEXT NOT NULL,
                cost_basis TEXT NOT NULL,
                PRIMARY KEY (symbol, market)
            );
            ",
        )?;
        Ok(())
    }

    // ── Settings ─────────────────────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn save_data_source(&self, provider: &str, akshare_url: &str) -> Result<()> {
        self.set_setting("data_source.provider", provider)?;
        self.set_setting("data_source.akshare_url", akshare_url)?;
        Ok(())
    }

    /// Returns `(provider, akshare_url)` if saved.
    pub fn load_data_source(&self) -> (Option<String>, Option<String>) {
        (
            self.get_setting("data_source.provider"),
            self.get_setting("data_source.akshare_url"),
        )
    }

    pub fn save_language(&self, lang: &str) -> Result<()> {
        self.set_setting("language", lang)
    }

    pub fn load_language(&self) -> Option<String> {
        self.get_setting("language")
    }

    pub fn save_zhitu_token(&self, token: &str) -> Result<()> {
        self.set_setting("zhitu.token", token)
    }

    pub fn load_zhitu_token(&self) -> Option<String> {
        self.get_setting("zhitu.token")
    }

    // ── Watchlist ─────────────────────────────────────────────────────────────

    pub fn load_watchlist(&self) -> Vec<Symbol> {
        let mut stmt = self
            .conn
            .prepare("SELECT symbol, market FROM watchlist ORDER BY position")
            .expect("prepare watchlist query");
        let items: Vec<Symbol> = stmt
            .query_map([], |row| {
                let sym: String = row.get(0)?;
                let mkt: String = row.get(1)?;
                Ok((sym, mkt))
            })
            .expect("query watchlist")
            .filter_map(|r| {
                let (sym, mkt) = r.ok()?;
                let market = mkt.parse::<Market>().ok()?;
                Some(Symbol::new(&sym, market))
            })
            .collect();
        crate::log_diag(&format!(
            "load_watchlist count={}: {:?}",
            items.len(),
            items.iter().map(|s| &s.code).collect::<Vec<_>>()
        ));
        items
    }

    pub fn add_to_watchlist(&self, symbol: &Symbol) -> Result<()> {
        let max_pos: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) FROM watchlist",
                [],
                |row| row.get(0),
            )
            .unwrap_or(-1);
        let affected = self.conn.execute(
            "INSERT OR IGNORE INTO watchlist (symbol, market, position) VALUES (?1, ?2, ?3)",
            params![symbol.code, market_key(&symbol.market), max_pos + 1],
        )?;
        crate::log_diag(&format!(
            "add_to_watchlist code={} market={} affected={}",
            symbol.code,
            market_key(&symbol.market),
            affected
        ));
        Ok(())
    }

    pub fn remove_from_watchlist(&self, symbol: &Symbol) -> Result<()> {
        let affected = self.conn.execute(
            "DELETE FROM watchlist WHERE symbol = ?1 AND market = ?2",
            params![symbol.code, market_key(&symbol.market)],
        )?;
        crate::log_diag(&format!(
            "remove_from_watchlist code={} market={} affected={}",
            symbol.code,
            market_key(&symbol.market),
            affected
        ));
        Ok(())
    }

    // ── Portfolio ─────────────────────────────────────────────────────────────

    pub fn load_portfolio(&self) -> Portfolio {
        let mut stmt = self
            .conn
            .prepare("SELECT symbol, market, quantity, cost_basis FROM portfolio")
            .expect("prepare portfolio query");
        let positions = stmt
            .query_map([], |row| {
                let sym: String = row.get(0)?;
                let mkt: String = row.get(1)?;
                let qty: String = row.get(2)?;
                let cost: String = row.get(3)?;
                Ok((sym, mkt, qty, cost))
            })
            .expect("query portfolio")
            .filter_map(|r| {
                let (sym, mkt, qty, cost) = r.ok()?;
                let market = mkt.parse::<Market>().ok()?;
                let quantity = Decimal::from_str(&qty).ok()?;
                let cost_basis = Decimal::from_str(&cost).ok()?;
                Some(Position {
                    symbol: Symbol::new(&sym, market),
                    quantity,
                    cost_basis,
                })
            })
            .collect();
        Portfolio { positions }
    }

    // ── Seeding ───────────────────────────────────────────────────────────────

    /// Populate watchlist and portfolio from TOML config if the DB tables are empty.
    /// Call this on first run to migrate from the old config file.
    pub fn seed_if_empty(&self, cfg: &crate::config::Config) -> Result<()> {
        let wl_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM watchlist", [], |row| row.get(0))
            .unwrap_or(0);
        if wl_count == 0 {
            for (pos, entry) in cfg.watchlist.iter().enumerate() {
                if let Ok(market) = entry.market.parse::<Market>() {
                    self.conn.execute(
                        "INSERT OR IGNORE INTO watchlist (symbol, market, position) VALUES (?1, ?2, ?3)",
                        params![entry.symbol, market_key(&market), pos as i64],
                    )?;
                }
            }
        }

        let port_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM portfolio", [], |row| row.get(0))
            .unwrap_or(0);
        if port_count == 0 {
            for entry in &cfg.portfolio {
                self.conn.execute(
                    "INSERT OR IGNORE INTO portfolio (symbol, market, quantity, cost_basis) VALUES (?1, ?2, ?3, ?4)",
                    params![entry.symbol, entry.market, entry.quantity, entry.cost_basis],
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::Market;

    fn in_memory() -> Storage {
        Storage::open(":memory:").expect("open :memory: db")
    }

    #[test]
    fn test_settings_roundtrip() {
        let s = in_memory();
        assert_eq!(s.get_setting("foo"), None);
        s.set_setting("foo", "bar").unwrap();
        assert_eq!(s.get_setting("foo").as_deref(), Some("bar"));
        // overwrite
        s.set_setting("foo", "baz").unwrap();
        assert_eq!(s.get_setting("foo").as_deref(), Some("baz"));
    }

    #[test]
    fn test_data_source_roundtrip() {
        let s = in_memory();
        assert_eq!(s.load_data_source(), (None, None));
        s.save_data_source("akshare", "http://127.0.0.1:9000").unwrap();
        assert_eq!(
            s.load_data_source(),
            (Some("akshare".into()), Some("http://127.0.0.1:9000".into()))
        );
        // overwrite
        s.save_data_source("sina", "http://127.0.0.1:8080").unwrap();
        assert_eq!(
            s.load_data_source(),
            (Some("sina".into()), Some("http://127.0.0.1:8080".into()))
        );
    }

    #[test]
    fn test_language_roundtrip() {
        let s = in_memory();
        assert_eq!(s.load_language(), None);
        s.save_language("en").unwrap();
        assert_eq!(s.load_language().as_deref(), Some("en"));
        s.save_language("zh").unwrap();
        assert_eq!(s.load_language().as_deref(), Some("zh"));
    }

    #[test]
    fn test_zhitu_token_roundtrip() {
        let s = in_memory();
        assert_eq!(s.load_zhitu_token(), None);
        s.save_zhitu_token("my-secret-token").unwrap();
        assert_eq!(s.load_zhitu_token().as_deref(), Some("my-secret-token"));
        // overwrite
        s.save_zhitu_token("new-token").unwrap();
        assert_eq!(s.load_zhitu_token().as_deref(), Some("new-token"));
    }

    #[test]
    fn test_watchlist_add_and_load() {
        let s = in_memory();
        assert!(s.load_watchlist().is_empty());
        let sym = Symbol::new("600519", Market::AShare);
        s.add_to_watchlist(&sym).unwrap();
        let wl = s.load_watchlist();
        assert_eq!(wl.len(), 1);
        assert_eq!(wl[0].code, "600519");
    }

    #[test]
    fn test_watchlist_remove() {
        let s = in_memory();
        let sym = Symbol::new("600519", Market::AShare);
        s.add_to_watchlist(&sym).unwrap();
        s.remove_from_watchlist(&sym).unwrap();
        assert!(s.load_watchlist().is_empty());
    }

    #[test]
    fn test_watchlist_duplicate_ignored() {
        let s = in_memory();
        let sym = Symbol::new("600519", Market::AShare);
        s.add_to_watchlist(&sym).unwrap();
        s.add_to_watchlist(&sym).unwrap(); // should not fail or duplicate
        assert_eq!(s.load_watchlist().len(), 1);
    }

    #[test]
    fn test_watchlist_preserves_order() {
        let s = in_memory();
        let syms = vec![
            Symbol::new("600519", Market::AShare),
            Symbol::new("000001", Market::AShare),
            Symbol::new("AAPL", Market::USStock),
        ];
        for sym in &syms {
            s.add_to_watchlist(sym).unwrap();
        }
        let loaded = s.load_watchlist();
        assert_eq!(loaded[0].code, "600519");
        assert_eq!(loaded[1].code, "000001");
        assert_eq!(loaded[2].code, "AAPL");
    }

    #[test]
    fn test_portfolio_roundtrip() {
        let s = in_memory();
        assert!(s.load_portfolio().positions.is_empty());
        s.conn
            .execute(
                "INSERT INTO portfolio (symbol, market, quantity, cost_basis) VALUES ('AAPL','us','10','1800.00')",
                [],
            )
            .unwrap();
        let port = s.load_portfolio();
        assert_eq!(port.positions.len(), 1);
        assert_eq!(port.positions[0].symbol.code, "AAPL");
    }

    #[test]
    fn test_seed_if_empty_populates_watchlist() {
        let s = in_memory();
        let cfg = crate::config::Config {
            watchlist: vec![crate::config::WatchlistEntry {
                symbol: "600519".into(),
                market: "a_share".into(),
            }],
            ..Default::default()
        };
        s.seed_if_empty(&cfg).unwrap();
        let wl = s.load_watchlist();
        assert_eq!(wl.len(), 1);
        assert_eq!(wl[0].code, "600519");

        // calling again should not duplicate
        s.seed_if_empty(&cfg).unwrap();
        assert_eq!(s.load_watchlist().len(), 1);
    }
}
