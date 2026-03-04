#[derive(Debug, Copy, Clone, PartialEq)]
pub struct BankMetricPoint {
    pub month: u64,
    pub reserve: f64,
    pub deposits: f64,
    pub created_money: f64,
    pub equity: f64,
    pub run_intensity: f64,
}
