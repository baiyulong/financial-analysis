pub(crate) mod portfolio;
pub mod engine;
pub mod result;
pub mod strategies;
pub mod strategy;

pub use engine::{BacktestConfig, Engine};
pub use result::{BacktestResult, Trade, TradeAction};
pub use strategies::BuiltinStrategy;
pub use strategy::{BarContext, Signal, Strategy};
