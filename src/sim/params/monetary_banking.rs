#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MonetaryPolicyParams {
    /// Annual CPI inflation target (2% = 0.02).
    pub target_inflation_rate: f64,
    /// Equilibrium policy rate when inflation is on target and unemployment is at NAIRU.
    pub neutral_policy_rate: f64,
    /// Taylor Rule inflation coefficient (α). Typically > 1 (Taylor principle).
    pub inflation_coefficient: f64,
    /// Taylor Rule unemployment gap coefficient (β).
    pub unemployment_coefficient: f64,
    /// Natural rate of unemployment (NAIRU).
    pub natural_unemployment_rate: f64,
    /// Zero lower bound — policy rate cannot fall below this.
    pub rate_floor: f64,
    /// Hard cap on policy rate.
    pub rate_ceiling: f64,
    /// Spread added to policy rate for private bank loans (credit spread).
    pub credit_spread: f64,
    /// Spread added to policy rate for sovereign bond coupon.
    pub sovereign_spread: f64,
    /// Government issues bonds when treasury_balance falls at or below this level.
    pub bond_issuance_treasury_threshold: f64,
    /// Fraction of each bond issuance absorbed by the bank as primary dealer.
    pub bank_primary_dealer_fraction: f64,
    /// Bond term in months.
    pub bond_maturity_months: u8,
    /// Top-N wealthiest people who absorb the retail portion of bond issuances/coupons.
    pub bond_retail_top_n: usize,
    /// Minimum absolute rate change needed to emit a RatePolicyChange event.
    pub rate_change_event_threshold: f64,
    /// Discrete step size for policy rate moves (0.0025 = 25 bps, like real central banks).
    pub policy_rate_step: f64,
    /// Deposit rate = max(0, policy_rate - deposit_spread). Models bank interest margin.
    pub deposit_spread: f64,
    /// Minimum cash balance (exclusive) required to earn deposit interest.
    pub savings_min_balance: f64,
    /// Absolute annual-rate gap from neutral loan rate that maps to a full policy response.
    pub rate_response_band: f64,
    /// Slope for loan appetite response to the normalized rate gap.
    pub loan_appetite_strength: f64,
    /// Minimum loan appetite multiplier when rates are high.
    pub loan_appetite_min: f64,
    /// Maximum loan appetite multiplier when rates are low.
    pub loan_appetite_max: f64,
    /// Slope for discretionary spending response to the normalized rate gap.
    pub discretionary_spend_strength: f64,
    /// Minimum discretionary spending multiplier when rates are high.
    pub discretionary_spend_min: f64,
    /// Maximum discretionary spending multiplier when rates are low.
    pub discretionary_spend_max: f64,
    /// Slope for company expansion and startup response to the normalized rate gap.
    pub expansion_strength: f64,
    /// Minimum expansion multiplier when rates are high.
    pub expansion_min: f64,
    /// Maximum expansion multiplier when rates are low.
    pub expansion_max: f64,
    /// Slope for firm pricing pressure response to the normalized rate gap.
    pub price_pressure_strength: f64,
    /// Minimum pricing-pressure multiplier when rates are high.
    pub price_pressure_min: f64,
    /// Maximum pricing-pressure multiplier when rates are low.
    pub price_pressure_max: f64,
    /// Maximum absolute shift applied to sell-through pricing thresholds.
    pub sell_through_shift_max: f64,
    /// Slope for sell-through threshold shift response to the normalized rate gap.
    pub sell_through_shift_strength: f64,
    /// Exponential smoothing factor for annualized month-over-month inflation.
    /// 1.0 = no smoothing, 0.0 = fully static.
    pub inflation_ema_alpha: f64,
    /// Weight applied to inflation shortfall when computing automatic minting.
    /// Contribution uses `max(target_inflation_rate - inflation, 0)`.
    pub mint_inflation_gap_weight: f64,
    /// Weight applied to unemployment gap when computing automatic minting.
    /// Contribution uses `max(unemployment - natural_unemployment_rate, 0)`.
    pub mint_unemployment_gap_weight: f64,
    /// Max monthly minted amount as a fraction of expected money supply.
    pub mint_max_fraction_expected_supply: f64,
    /// Multiplier on `target_inflation_rate` forming the hard cap: if
    /// `monthly_inflation_rate > target * multiplier`, unemployment minting is scaled down.
    pub mint_unemployment_inflation_cap_multiplier: f64,
    /// Fraction of unemployment gap preserved even during inflation overshoot (0.0 = current behavior).
    /// When inflation exceeds the cap, unemployment_gap is multiplied by this fraction instead of zeroing.
    pub mint_unemployment_inflation_floor_fraction: f64,
    /// EMA alpha for smoothing raw monthly CPI change before annualization (0.0 = fully smoothed, 1.0 = no smoothing).
    /// Provides a first-stage dampening layer to reduce CPI volatility from the annualization step.
    pub cpi_monthly_change_ema_alpha: f64,
}

impl Default for MonetaryPolicyParams {
    fn default() -> Self {
        Self {
            target_inflation_rate: 0.02,
            neutral_policy_rate: 0.04,
            inflation_coefficient: 1.5,
            unemployment_coefficient: 0.5,
            natural_unemployment_rate: 0.03,
            rate_floor: 0.0,
            rate_ceiling: 0.20,
            credit_spread: 0.03,
            sovereign_spread: 0.01,
            bond_issuance_treasury_threshold: 0.0,
            bank_primary_dealer_fraction: 0.50,
            bond_maturity_months: 12,
            bond_retail_top_n: 200,
            rate_change_event_threshold: 0.005,
            policy_rate_step: 0.0025,
            deposit_spread: 0.01,
            savings_min_balance: 200.0,
            rate_response_band: 0.06,
            loan_appetite_strength: 0.55,
            loan_appetite_min: 0.55,
            loan_appetite_max: 1.35,
            discretionary_spend_strength: 0.45,
            discretionary_spend_min: 0.65,
            discretionary_spend_max: 1.25,
            expansion_strength: 0.60,
            expansion_min: 0.45,
            expansion_max: 1.45,
            price_pressure_strength: 0.70,
            price_pressure_min: 0.35,
            price_pressure_max: 1.20,
            sell_through_shift_max: 0.05,
            sell_through_shift_strength: 0.04,
            inflation_ema_alpha: 0.35,
            mint_inflation_gap_weight: 0.20,
            mint_unemployment_gap_weight: 0.10,
            mint_max_fraction_expected_supply: 0.01,
            mint_unemployment_inflation_cap_multiplier: 1.5,
            mint_unemployment_inflation_floor_fraction: 0.0,
            cpi_monthly_change_ema_alpha: 0.3,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BankingSystemParams {
    pub deposit_target_ratio: f64,
    pub wallet_buffer_months_person: f64,
    pub wallet_buffer_months_company: f64,
    pub deposit_insurance_cap: f64,
    pub uninsured_haircut_rate: f64,
    /// Health score below which private banks may enter run dynamics.
    pub run_health_threshold: f64,
    /// Fraction of deposits requested during a severe run.
    pub run_withdraw_fraction: f64,
    /// Fraction of depositors that attempt to switch away during a run.
    pub run_switch_fraction: f64,
    /// Legacy stability nudging factor retained for compatibility.
    pub public_bank_stability_bonus: f64,
    pub private_bank_failure_sensitivity: f64,
    /// Minimum uninsured-share fraction needed before run dynamics can start.
    pub run_min_uninsured_share_trigger: f64,
    /// Liquidity floor below which a shortfall can push a bank into resolution.
    pub resolution_liquidity_floor: f64,
    /// Consecutive months of distress required before entering resolution.
    pub resolution_entry_consecutive_months: u16,
    /// Consecutive stable months required before exiting resolution.
    pub resolution_exit_consecutive_months: u16,
    /// Fraction of run intensity that decays each stable month.
    pub run_intensity_decay_monthly: f64,
    /// Consecutive months in resolution after which a private bank is terminally closed.
    pub terminal_resolution_months: u16,
    pub min_private_bank_founders: usize,
    /// Minimum founder cash kept after seed contribution.
    pub min_founder_cash_after_contribution: f64,
    pub min_private_bank_seed_capital: f64,
    /// Minimum ratio seed_capital / migrated_deposits for a new private bank.
    pub min_private_seed_to_deposits_ratio: f64,
    /// Target residents per private bank used as a soft market-capacity gate.
    pub target_people_per_private_bank: usize,
    pub max_new_banks_per_month: u16,
    /// Amount of capital injected when recapitalizing the public bank.
    pub public_bank_recapitalize_amount: f64,
    /// Minimum health floor for sovereign-backed public banks.
    pub public_health_floor: f64,
    /// Public-bank health score weight on liquidity.
    pub public_health_liquidity_weight: f64,
    /// Public-bank health score weight on capital.
    pub public_health_capital_weight: f64,
    /// Public-bank health score weight on sovereign support capacity.
    pub public_health_sovereign_support_weight: f64,
    // --- Cross-country deposits ---
    /// Minimum local cash balance required for a person to open a foreign deposit.
    pub cross_country_deposit_min_balance: f64,
    /// Minimum annual rate advantage (foreign - local) to trigger deposit migration.
    pub cross_country_deposit_rate_premium: f64,
    /// Fraction of savings above the minimum balance moved to a foreign bank.
    pub cross_country_deposit_fraction: f64,
    /// Cash balance below which a person recalls their foreign deposit (distress).
    pub cross_country_deposit_distress_threshold: f64,
    // --- Bank employment ---
    /// Job type IDs that banks hire (e.g. Office Clerk=2, Software Dev=9).
    pub bank_employee_job_types: [u8; 2],
    /// One employee per this many dollars of total AUM (deposits + loans + equity book value).
    pub bank_employees_per_deposit_unit: f64,
    /// Minimum staff even for a tiny bank.
    pub bank_min_employees: u32,
    /// Hard cap on bank headcount (safety valve for extreme AUM growth).
    pub bank_max_employees: u32,
    /// Banks pay this multiple of the catalog salary (1.15 = 15% premium).
    pub bank_wage_multiplier: f64,
    /// How many private banks to pre-seed at world generation (bypasses formation ratio check).
    pub initial_private_banks: u8,
    /// Deposit rate bonus for private banks (above base rate) to compete with the public bank.
    pub private_bank_deposit_rate_bonus: f64,
}

impl Default for BankingSystemParams {
    fn default() -> Self {
        Self {
            deposit_target_ratio: 0.85,
            wallet_buffer_months_person: 0.20,
            wallet_buffer_months_company: 0.25,
            deposit_insurance_cap: 100_000.0,
            uninsured_haircut_rate: 0.35,
            run_health_threshold: 0.16,
            run_withdraw_fraction: 0.12,
            run_switch_fraction: 0.24,
            public_bank_stability_bonus: 0.10,
            private_bank_failure_sensitivity: 0.22,
            run_min_uninsured_share_trigger: 0.30,
            resolution_liquidity_floor: 0.03,
            resolution_entry_consecutive_months: 2,
            resolution_exit_consecutive_months: 2,
            run_intensity_decay_monthly: 0.75,
            terminal_resolution_months: 12,
            min_private_bank_founders: 4,
            min_founder_cash_after_contribution: 12_500.0,
            min_private_bank_seed_capital: 200_000.0,
            min_private_seed_to_deposits_ratio: 0.06,
            target_people_per_private_bank: 60_000,
            max_new_banks_per_month: 3,
            public_bank_recapitalize_amount: 500_000.0,
            public_health_floor: 0.40,
            public_health_liquidity_weight: 0.55,
            public_health_capital_weight: 0.15,
            public_health_sovereign_support_weight: 0.30,
            cross_country_deposit_min_balance: 10_000.0,
            cross_country_deposit_rate_premium: 0.005,
            cross_country_deposit_fraction: 0.30,
            cross_country_deposit_distress_threshold: 500.0,
            bank_employee_job_types: [2, 9],
            bank_employees_per_deposit_unit: 5_000_000.0,
            bank_min_employees: 2,
            bank_max_employees: 500,
            bank_wage_multiplier: 1.15,
            initial_private_banks: 1,
            private_bank_deposit_rate_bonus: 0.010,
        }
    }
}
