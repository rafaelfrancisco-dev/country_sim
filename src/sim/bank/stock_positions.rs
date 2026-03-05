use super::{Bank, BankSecurityId, BankStockPosition};
use crate::sim::money::round_cents;

impl Bank {
    pub fn stock_position_shares(&self, security_id: BankSecurityId) -> u64 {
        self.stock_positions
            .get(&security_id)
            .map_or(0, |position| position.shares)
    }

    pub fn add_stock_shares(
        &mut self,
        security_id: BankSecurityId,
        shares: u64,
        cost_per_share: f64,
    ) {
        if shares == 0 {
            return;
        }
        let cost_per_share = round_cents(cost_per_share.max(0.0));
        if let Some(position) = self.stock_positions.get_mut(&security_id) {
            let total_cost =
                position.avg_cost * position.shares as f64 + cost_per_share * shares as f64;
            position.shares = position.shares.saturating_add(shares);
            position.avg_cost = if position.shares == 0 {
                0.0
            } else {
                round_cents(total_cost / position.shares as f64)
            };
            return;
        }
        self.stock_positions.insert(
            security_id,
            BankStockPosition {
                security_id,
                shares,
                avg_cost: cost_per_share,
            },
        );
    }
}
