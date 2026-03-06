use std::{
    fs::{self, File},
    io::BufWriter,
    path::Path,
};

use crate::sim::world::{
    CountryExportData, StockDailyCountryExportData, StockDailyEquityExportData,
};

pub struct CsvExporter {
    country_metrics: csv::Writer<BufWriter<File>>,
    people_agg: csv::Writer<BufWriter<File>>,
    company_agg: csv::Writer<BufWriter<File>>,
    events: csv::Writer<BufWriter<File>>,
    stock_daily_country: csv::Writer<BufWriter<File>>,
    stock_daily_equity: csv::Writer<BufWriter<File>>,
}

impl CsvExporter {
    pub fn new(dir: &Path, seed: u64) -> std::io::Result<Self> {
        fs::create_dir_all(dir)?;

        let mut country_metrics = csv::Writer::from_writer(BufWriter::new(File::create(
            dir.join(format!("country_metrics_{seed:016x}.csv")),
        )?));
        let mut people_agg = csv::Writer::from_writer(BufWriter::new(File::create(
            dir.join(format!("people_agg_{seed:016x}.csv")),
        )?));
        let mut company_agg = csv::Writer::from_writer(BufWriter::new(File::create(
            dir.join(format!("company_agg_{seed:016x}.csv")),
        )?));
        let mut events = csv::Writer::from_writer(BufWriter::new(File::create(
            dir.join(format!("events_{seed:016x}.csv")),
        )?));
        let mut stock_daily_country = csv::Writer::from_writer(BufWriter::new(File::create(
            dir.join(format!("stock_daily_country_{seed:016x}.csv")),
        )?));
        let mut stock_daily_equity = csv::Writer::from_writer(BufWriter::new(File::create(
            dir.join(format!("stock_daily_equity_{seed:016x}.csv")),
        )?));

        country_metrics.write_record([
            "month",
            "country_id",
            "country_name",
            "population",
            "treasury_balance",
            "monthly_tax_collected",
            "monthly_tariff_collected",
            "monthly_gov_spending",
            "monthly_budget_balance",
            "gov_debt",
            "monthly_household_spending",
            "monthly_goods_import_spending",
            "monthly_goods_export_revenue",
            "monthly_unmet_need_events",
            "monthly_severe_unmet_need_events",
            "monthly_chronic_unmet_need_count",
            "monthly_essential_debt_spending",
            "water_unmet_share_pct",
            "water_severe_unmet_share_pct",
            "water_producer_count",
            "water_public_utility_count",
            "water_inventory_cover_months",
            "food_unmet_share_pct",
            "food_severe_unmet_share_pct",
            "food_producer_count",
            "food_public_utility_count",
            "food_inventory_cover_months",
            "housing_unmet_share_pct",
            "housing_severe_unmet_share_pct",
            "housing_producer_count",
            "housing_public_utility_count",
            "housing_inventory_cover_months",
            "electricity_unmet_share_pct",
            "electricity_severe_unmet_share_pct",
            "electricity_producer_count",
            "electricity_public_utility_count",
            "electricity_inventory_cover_months",
            "medicine_unmet_share_pct",
            "medicine_severe_unmet_share_pct",
            "medicine_producer_count",
            "medicine_public_utility_count",
            "medicine_inventory_cover_months",
            "total_money_supply",
            "household_cash_total",
            "company_cash_total",
            "bank_reserve_balance",
            "bank_created_money_outstanding",
            "expected_money_supply",
            "money_supply_gap",
            "monthly_money_minted",
            "cumulative_money_minted",
            "bank_count_total",
            "bank_count_public",
            "bank_count_private",
            "bank_health_avg_pct",
            "bank_health_min_pct",
            "bank_health_max_pct",
            "bank_health_public_pct",
            "bank_health_private_avg_pct",
            "bank_health_private_min_pct",
            "bank_health_private_max_pct",
            "bank_health_below_50_count",
            "bank_health_private_below_50_count",
            "private_bank_deposit_liabilities",
            "private_bank_deposit_share_pct",
            "private_people_depositor_share_pct",
            "private_company_depositor_share_pct",
            "monthly_private_banks_founded",
            "cumulative_private_banks_founded",
            "monthly_private_banks_closed",
            "cumulative_private_banks_closed",
            "bonds_outstanding",
            "bond_coupon_rate",
            "monthly_coupon_paid",
            "monthly_bonds_issued",
            "monthly_bonds_matured",
            "cpi_index",
            "inflation_rate",
            "policy_rate",
        ])?;

        people_agg.write_record([
            "month",
            "country_id",
            "country_name",
            "population",
            "employed_count",
            "unemployed_count",
            "labor_force_count",
            "non_participating_count",
            "unemployment_rate_pct",
            "participation_rate_pct",
            "headline_non_employed_rate_pct",
            "avg_age_years",
            "avg_cash_balance",
            "total_gross_pay",
            "total_tax_paid",
            "total_goods_spending",
            "count_with_active_loans",
            "avg_loan_balance",
            "count_with_unmet_needs",
            "count_with_severe_unmet_needs",
            "count_with_chronic_unmet_needs",
        ])?;

        company_agg.write_record([
            "month",
            "country_id",
            "country_name",
            "total_companies",
            "total_employees",
            "total_open_positions",
            "total_company_cash",
            "total_revenue",
            "total_payroll",
            "total_profit",
            "count_profitable",
            "count_unprofitable",
            "total_units_produced",
            "total_units_sold",
            "total_units_exported",
            "count_public_utility",
            "count_backfill",
            "count_companies_with_loans",
        ])?;

        events.write_record([
            "month",
            "country_id",
            "country_name",
            "births",
            "deaths",
            "job_switches",
            "layoffs_total",
            "separations_quits",
            "separations_retirements",
            "separations_layoffs_non_distress",
            "separations_total",
            "companies_founded",
            "banks_founded",
            "banks_closed",
            "bank_runs_started",
            "bank_resolutions_started",
            "bankruptcies",
            "wage_cuts",
            "loans_issued",
            "loans_ended",
            "loans_defaulted",
            "debt_distress_events",
            "gov_transfers",
            "hires_total",
            "stock_ipos",
            "stock_delistings",
            "stock_margin_calls",
        ])?;

        stock_daily_country.write_record([
            "day",
            "country_id",
            "country_name",
            "listed_count",
            "trades_count",
            "total_volume",
            "total_turnover",
            "index_value",
            "bank_equity_exposure",
            "bank_reserve_balance",
            "active_traders_count",
            "buy_orders_count",
            "sell_orders_count",
        ])?;

        stock_daily_equity.write_record([
            "day",
            "country_id",
            "country_name",
            "company_id",
            "ticker",
            "close_price",
            "day_return_pct",
            "volume",
            "turnover",
            "trades_count",
            "bank_position_shares",
            "free_float_shares",
        ])?;

        country_metrics.flush()?;
        people_agg.flush()?;
        company_agg.flush()?;
        events.flush()?;
        stock_daily_country.flush()?;
        stock_daily_equity.flush()?;

        Ok(Self {
            country_metrics,
            people_agg,
            company_agg,
            events,
            stock_daily_country,
            stock_daily_equity,
        })
    }

    pub fn write_row(&mut self, data: &CountryExportData) -> std::io::Result<()> {
        let unemployment_rate_pct = if data.labor_force_count > 0 {
            data.unemployed_count as f64 / data.labor_force_count as f64 * 100.0
        } else {
            0.0
        };
        let participation_rate_pct = if data.population > 0 {
            data.labor_force_count as f64 / data.population as f64 * 100.0
        } else {
            0.0
        };

        self.country_metrics.write_record([
            data.month.to_string(),
            data.id.to_string(),
            data.name.clone(),
            data.population.to_string(),
            format!("{:.2}", data.treasury_balance),
            format!("{:.2}", data.monthly_tax_collected),
            format!("{:.2}", data.monthly_tariff_collected),
            format!("{:.2}", data.monthly_gov_spending),
            format!("{:.2}", data.monthly_budget_balance),
            format!("{:.2}", data.gov_debt),
            format!("{:.2}", data.monthly_household_spending),
            format!("{:.2}", data.monthly_goods_import_spending),
            format!("{:.2}", data.monthly_goods_export_revenue),
            data.monthly_unmet_need_events.to_string(),
            data.monthly_severe_unmet_need_events.to_string(),
            data.monthly_chronic_unmet_need_count.to_string(),
            format!("{:.2}", data.monthly_essential_debt_spending),
            format!("{:.2}", data.essential_goods[0].unmet_share_pct),
            format!("{:.2}", data.essential_goods[0].severe_unmet_share_pct),
            data.essential_goods[0].producer_count.to_string(),
            data.essential_goods[0].public_utility_count.to_string(),
            format!("{:.2}", data.essential_goods[0].inventory_cover_months),
            format!("{:.2}", data.essential_goods[1].unmet_share_pct),
            format!("{:.2}", data.essential_goods[1].severe_unmet_share_pct),
            data.essential_goods[1].producer_count.to_string(),
            data.essential_goods[1].public_utility_count.to_string(),
            format!("{:.2}", data.essential_goods[1].inventory_cover_months),
            format!("{:.2}", data.essential_goods[2].unmet_share_pct),
            format!("{:.2}", data.essential_goods[2].severe_unmet_share_pct),
            data.essential_goods[2].producer_count.to_string(),
            data.essential_goods[2].public_utility_count.to_string(),
            format!("{:.2}", data.essential_goods[2].inventory_cover_months),
            format!("{:.2}", data.essential_goods[3].unmet_share_pct),
            format!("{:.2}", data.essential_goods[3].severe_unmet_share_pct),
            data.essential_goods[3].producer_count.to_string(),
            data.essential_goods[3].public_utility_count.to_string(),
            format!("{:.2}", data.essential_goods[3].inventory_cover_months),
            format!("{:.2}", data.essential_goods[4].unmet_share_pct),
            format!("{:.2}", data.essential_goods[4].severe_unmet_share_pct),
            data.essential_goods[4].producer_count.to_string(),
            data.essential_goods[4].public_utility_count.to_string(),
            format!("{:.2}", data.essential_goods[4].inventory_cover_months),
            format!("{:.2}", data.total_money_supply),
            format!("{:.2}", data.household_cash_total),
            format!("{:.2}", data.company_cash_total),
            format!("{:.2}", data.bank_reserve_balance),
            format!("{:.2}", data.bank_created_money_outstanding),
            format!("{:.2}", data.expected_money_supply),
            format!("{:.2}", data.money_supply_gap),
            format!("{:.2}", data.monthly_money_minted),
            format!("{:.2}", data.cumulative_money_minted),
            data.bank_count_total.to_string(),
            data.bank_count_public.to_string(),
            data.bank_count_private.to_string(),
            format!("{:.4}", data.bank_health_avg_pct),
            format!("{:.4}", data.bank_health_min_pct),
            format!("{:.4}", data.bank_health_max_pct),
            format!("{:.4}", data.bank_health_public_pct),
            format!("{:.4}", data.bank_health_private_avg_pct),
            format!("{:.4}", data.bank_health_private_min_pct),
            format!("{:.4}", data.bank_health_private_max_pct),
            data.bank_health_below_50_count.to_string(),
            data.bank_health_private_below_50_count.to_string(),
            format!("{:.2}", data.private_bank_deposit_liabilities),
            format!("{:.4}", data.private_bank_deposit_share_pct),
            format!("{:.4}", data.private_people_depositor_share_pct),
            format!("{:.4}", data.private_company_depositor_share_pct),
            data.monthly_private_banks_founded.to_string(),
            data.cumulative_private_banks_founded.to_string(),
            data.monthly_private_banks_closed.to_string(),
            data.cumulative_private_banks_closed.to_string(),
            format!("{:.2}", data.bonds_outstanding),
            format!("{:.6}", data.bond_coupon_rate),
            format!("{:.2}", data.monthly_coupon_paid),
            format!("{:.2}", data.monthly_bonds_issued),
            format!("{:.2}", data.monthly_bonds_matured),
            format!("{:.6}", data.cpi_index),
            format!("{:.6}", data.inflation_rate),
            format!("{:.6}", data.policy_rate),
        ])?;

        self.people_agg.write_record([
            data.month.to_string(),
            data.id.to_string(),
            data.name.clone(),
            data.population.to_string(),
            data.employed_count.to_string(),
            data.unemployed_count.to_string(),
            data.labor_force_count.to_string(),
            data.non_participating_count.to_string(),
            format!("{:.4}", unemployment_rate_pct),
            format!("{:.4}", participation_rate_pct),
            format!("{:.4}", data.headline_non_employed_rate_pct),
            format!("{:.4}", data.avg_age_years),
            format!("{:.2}", data.avg_cash_balance),
            format!("{:.2}", data.total_gross_pay),
            format!("{:.2}", data.total_tax_paid),
            format!("{:.2}", data.total_goods_spending),
            data.count_with_active_loans.to_string(),
            format!("{:.2}", data.avg_loan_balance),
            data.count_with_unmet_needs.to_string(),
            data.count_with_severe_unmet_needs.to_string(),
            data.count_with_chronic_unmet_needs.to_string(),
        ])?;

        self.company_agg.write_record([
            data.month.to_string(),
            data.id.to_string(),
            data.name.clone(),
            data.total_companies.to_string(),
            data.total_employees.to_string(),
            data.total_open_positions.to_string(),
            format!("{:.2}", data.total_company_cash),
            format!("{:.2}", data.total_revenue),
            format!("{:.2}", data.total_payroll),
            format!("{:.2}", data.total_profit),
            data.count_profitable.to_string(),
            data.count_unprofitable.to_string(),
            data.total_units_produced.to_string(),
            data.total_units_sold.to_string(),
            data.total_units_exported.to_string(),
            data.count_public_utility.to_string(),
            data.count_backfill.to_string(),
            data.count_companies_with_loans.to_string(),
        ])?;

        self.events.write_record([
            data.month.to_string(),
            data.id.to_string(),
            data.name.clone(),
            data.births.to_string(),
            data.deaths.to_string(),
            data.job_switches.to_string(),
            data.layoffs_total.to_string(),
            data.separations_quits.to_string(),
            data.separations_retirements.to_string(),
            data.separations_layoffs_non_distress.to_string(),
            data.separations_total.to_string(),
            data.companies_founded.to_string(),
            data.banks_founded.to_string(),
            data.banks_closed.to_string(),
            data.bank_runs_started.to_string(),
            data.bank_resolutions_started.to_string(),
            data.bankruptcies.to_string(),
            data.wage_cuts.to_string(),
            data.loans_issued.to_string(),
            data.loans_ended.to_string(),
            data.loans_defaulted.to_string(),
            data.debt_distress_events.to_string(),
            data.gov_transfers.to_string(),
            data.hires_total.to_string(),
            data.stock_ipos.to_string(),
            data.stock_delistings.to_string(),
            data.stock_margin_calls.to_string(),
        ])?;

        self.country_metrics.flush()?;
        self.people_agg.flush()?;
        self.company_agg.flush()?;
        self.events.flush()?;

        Ok(())
    }

    pub fn write_stock_country_row(
        &mut self,
        data: &StockDailyCountryExportData,
    ) -> std::io::Result<()> {
        self.stock_daily_country.write_record([
            data.day.to_string(),
            data.country_id.to_string(),
            data.country_name.clone(),
            data.listed_count.to_string(),
            data.trades_count.to_string(),
            data.total_volume.to_string(),
            format!("{:.2}", data.total_turnover),
            format!("{:.4}", data.index_value),
            format!("{:.2}", data.bank_equity_exposure),
            format!("{:.2}", data.bank_reserve_balance),
            data.active_traders_count.to_string(),
            data.buy_orders_count.to_string(),
            data.sell_orders_count.to_string(),
        ])?;
        self.stock_daily_country.flush()?;
        Ok(())
    }

    pub fn write_stock_equity_row(
        &mut self,
        data: &StockDailyEquityExportData,
    ) -> std::io::Result<()> {
        self.stock_daily_equity.write_record([
            data.day.to_string(),
            data.country_id.to_string(),
            data.country_name.clone(),
            data.company_id.to_string(),
            data.ticker.clone(),
            format!("{:.4}", data.close_price),
            format!("{:.4}", data.day_return_pct),
            data.volume.to_string(),
            format!("{:.2}", data.turnover),
            data.trades_count.to_string(),
            data.bank_position_shares.to_string(),
            data.free_float_shares.to_string(),
        ])?;
        self.stock_daily_equity.flush()?;
        Ok(())
    }
}
