use crate::strategy::{BarContext, Signal, Strategy};

pub struct BollingerStrategy { 
    pub period: usize, 
    pub std_dev: f64 
}

impl BollingerStrategy {
    pub fn new(period: usize, std_dev: f64) -> Self { 
        Self { period, std_dev } 
    }
}

impl Strategy for BollingerStrategy {
    fn name(&self) -> &str { "布林带" }
    fn on_bar(&mut self, _ctx: &BarContext) -> Signal { Signal::Hold }
}
