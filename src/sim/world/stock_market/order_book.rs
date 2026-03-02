// ---------------------------------------------------------------------------
// Order book types
// ---------------------------------------------------------------------------

/// Identifies a participant in a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderId {
    Person(u32),
    Bank,
}

impl TraderId {
    /// Stable key for deterministic tie-breaking in sort.
    pub(in crate::sim::world) fn stable_key(self) -> u64 {
        match self {
            TraderId::Person(id) => id as u64,
            TraderId::Bank => u64::MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct LimitOrder {
    pub trader: TraderId,
    #[allow(dead_code)]
    pub company_id: u16,
    #[allow(dead_code)]
    pub side: OrderSide,
    /// Maximum price a buyer will pay / minimum price a seller will accept.
    pub limit_price: f64,
    pub remaining_shares: u64,
    /// Tracks remaining cash committed for this buy order (0.0 for sells).
    pub remaining_cash_budget: f64,
}

#[derive(Debug, Clone)]
pub struct TradeFill {
    pub company_id: u16,
    pub buyer: TraderId,
    pub seller: TraderId,
    pub shares: u64,
    pub price: f64,
}

// ---------------------------------------------------------------------------
// World entry point
// ---------------------------------------------------------------------------




