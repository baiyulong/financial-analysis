pub mod bollinger;
pub mod ma_cross;
pub mod rsi;

use crate::strategy::Strategy;
use ma_cross::MaCrossStrategy;

/// Enumeration of all built-in strategies.
#[derive(Clone, Debug)]
pub enum BuiltinStrategy {
    MaCross { fast: usize, slow: usize },
    Rsi { period: usize },
    Bollinger { period: usize },
}

impl BuiltinStrategy {
    pub fn all() -> Vec<BuiltinStrategy> {
        vec![
            BuiltinStrategy::MaCross { fast: 5, slow: 20 },
            BuiltinStrategy::Rsi { period: 14 },
            BuiltinStrategy::Bollinger { period: 20 },
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            BuiltinStrategy::MaCross { .. } => "双均线穿越 (MA5×MA20)",
            BuiltinStrategy::Rsi { .. } => "RSI 均值回归 (14/30/70)",
            BuiltinStrategy::Bollinger { .. } => "布林带 (20, 2σ)",
        }
    }

    pub fn to_boxed(&self) -> Box<dyn Strategy> {
        match self {
            BuiltinStrategy::MaCross { fast, slow } => {
                Box::new(MaCrossStrategy::new(*fast, *slow))
            }
            BuiltinStrategy::Rsi { period } => {
                Box::new(rsi::RsiStrategy::new(*period, 30, 70))
            }
            BuiltinStrategy::Bollinger { period } => {
                Box::new(bollinger::BollingerStrategy::new(*period, 2.0))
            }
        }
    }
}
