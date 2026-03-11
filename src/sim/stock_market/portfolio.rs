use crate::sim::money::round_cents;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StockSecurityId {
    pub country_id: u16,
    pub company_id: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub security_id: StockSecurityId,
    pub shares: u64,
    pub avg_cost: f64,
    pub pledged_shares: u64,
}

impl Position {
    pub fn market_value(&self, last_price: f64) -> f64 {
        self.shares as f64 * last_price
    }

    pub fn unrealized_pnl(&self, last_price: f64) -> f64 {
        self.market_value(last_price) - (self.shares as f64 * self.avg_cost)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Portfolio {
    pub positions: Vec<Position>,
    pub realized_pnl_mtd: f64,
    pub dividends_mtd: f64,
    pub last_completed_month_dividends: f64,
    pub last_completed_month_realized_pnl: f64,
}

impl Portfolio {
    pub fn position(&self, security_id: StockSecurityId) -> Option<&Position> {
        self.positions
            .iter()
            .find(|position| position.security_id == security_id)
    }

    pub fn add_shares(&mut self, security_id: StockSecurityId, shares: u64, cost_per_share: f64) {
        if shares == 0 {
            return;
        }
        if let Some(position) = self
            .positions
            .iter_mut()
            .find(|position| position.security_id == security_id)
        {
            let total_cost =
                position.shares as f64 * position.avg_cost + shares as f64 * cost_per_share;
            position.shares += shares;
            position.avg_cost = if position.shares == 0 {
                0.0
            } else {
                total_cost / position.shares as f64
            };
            return;
        }
        self.positions.push(Position {
            security_id,
            shares,
            avg_cost: cost_per_share,
            pledged_shares: 0,
        });
    }

    pub fn remove_shares(
        &mut self,
        security_id: StockSecurityId,
        shares: u64,
        sale_price: f64,
    ) -> f64 {
        let Some(position) = self
            .positions
            .iter_mut()
            .find(|position| position.security_id == security_id)
        else {
            return 0.0;
        };
        let removed = shares.min(position.shares);
        let pnl = round_cents((sale_price - position.avg_cost) * removed as f64);
        self.realized_pnl_mtd = round_cents(self.realized_pnl_mtd + pnl);
        position.shares -= removed;
        position.pledged_shares = position.pledged_shares.min(position.shares);
        if position.shares == 0 {
            self.positions
                .retain(|candidate| candidate.security_id != security_id);
        }
        pnl
    }

    pub fn wipe_security(&mut self, security_id: StockSecurityId) -> u64 {
        let Some(idx) = self
            .positions
            .iter()
            .position(|position| position.security_id == security_id)
        else {
            return 0;
        };
        let position = &self.positions[idx];
        let shares = position.shares;
        let loss = round_cents(-(position.shares as f64 * position.avg_cost));
        self.realized_pnl_mtd = round_cents(self.realized_pnl_mtd + loss);
        self.positions.swap_remove(idx);
        shares
    }

    pub fn total_equity_value(&self, market: &super::StockMarket) -> f64 {
        self.positions
            .iter()
            .map(|position| {
                let price = market
                    .listed
                    .get(&position.security_id.company_id)
                    .map_or(0.0, |equity| equity.last_price);
                position.shares as f64 * price
            })
            .sum()
    }

    pub fn pledge_all(&mut self) {
        for position in &mut self.positions {
            position.pledged_shares = position.shares;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}
