use super::*;
use crate::sim::good::good_by_id;

/// Runs a 2-year generated world and prints per-sector profitability.
/// Run with: cargo test profitability_probe -- --nocapture --ignored
#[test]
#[ignore = "slow integration probe; run on demand"]
fn profitability_probe_two_years() {
    let world = run_generated_world_in_pool(0x5eedc0de, 3, 2000, 720, 4);

    for country in &world.countries {
        let private: Vec<_> = country
            .companies
            .iter()
            .filter(|c| !c.is_public_utility)
            .collect();

        let total = private.len();
        if total == 0 {
            continue;
        }

        // Use last_completed_month_profit — last_month_profit resets mid-month.
        let profitable = private
            .iter()
            .filter(|c| c.last_completed_month_profit > 0.0)
            .count();

        eprintln!(
            "\n=== {} (day {}) ===",
            country.name, country.day
        );
        eprintln!(
            "  Private companies: {total}  profitable: {profitable}  ({:.1}%)",
            100.0 * profitable as f64 / total as f64
        );

        // Per-sector breakdown
        let mut by_good: std::collections::BTreeMap<u8, (usize, usize, f64, f64)> =
            std::collections::BTreeMap::new();
        for c in &private {
            let e = by_good.entry(c.produced_good_id).or_default();
            e.0 += 1;
            if c.last_completed_month_profit > 0.0 {
                e.1 += 1;
            }
            e.2 += c.last_completed_month_profit;
            e.3 += c.unit_price;
        }

        eprintln!("  {:<22} {:>5} {:>5} {:>8} {:>10}", "Sector", "total", "prof", "pct%", "avg price");
        for (good_id, (n, p, profit_sum, price_sum)) in &by_good {
            let name = good_by_id(*good_id).map_or("?", |g| g.name);
            eprintln!(
                "  {:<22} {:>5} {:>5} {:>7.1}% {:>10.2}  avg_profit:{:>10.0}",
                name,
                n,
                p,
                100.0 * *p as f64 / *n as f64,
                price_sum / *n as f64,
                profit_sum / *n as f64,
            );
        }
    }
}
