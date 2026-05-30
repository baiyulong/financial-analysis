use crate::strategy::{BarContext, Signal, Strategy};

pub struct RsiStrategy { 
    pub period: usize, 
    pub oversold: u32, 
    pub overbought: u32 
}

impl RsiStrategy {
    pub fn new(period: usize, oversold: u32, overbought: u32) -> Self {
        Self { period, oversold, overbought }
    }
}

impl Strategy for RsiStrategy {
    fn name(&self) -> &str { "RSI 均值回归" }
    fn on_bar(&mut self, _ctx: &BarContext) -> Signal { Signal::Hold }
}
