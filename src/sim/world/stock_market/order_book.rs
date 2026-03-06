// ---------------------------------------------------------------------------
// Order book types
// ---------------------------------------------------------------------------

/// Identifies a participant in a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderId {
    Person {
        country_id: u16,
        person_id: u32,
    },
    Bank {
        country_id: u16,
        bank_id: crate::sim::bank::BankId,
    },
}

impl TraderId {
    /// Stable key for deterministic tie-breaking in sort.
    pub(in crate::sim::world) fn stable_key(self) -> u64 {
        match self {
            TraderId::Person {
                country_id,
                person_id,
            } => (u64::from(country_id) << 32) | u64::from(person_id),
            TraderId::Bank {
                country_id,
                bank_id,
            } => (u64::from(country_id) << 32) | (u64::from(bank_id) << 16) | u64::MAX >> 16,
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
    pub security_id: crate::sim::stock_market::StockSecurityId,
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
    pub security_id: crate::sim::stock_market::StockSecurityId,
    pub buyer: TraderId,
    pub seller: TraderId,
    pub shares: u64,
    pub price: f64,
}

// ---------------------------------------------------------------------------
// World entry point
// ---------------------------------------------------------------------------
