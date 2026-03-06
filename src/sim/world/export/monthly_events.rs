use crate::sim::event::SimEvent;

#[derive(Default)]
pub(super) struct EventCounts {
    pub(super) births: u64,
    pub(super) deaths: u64,
    pub(super) job_switches: u64,
    pub(super) layoffs_total: u64,
    pub(super) separations_quits: u64,
    pub(super) separations_retirements: u64,
    pub(super) separations_layoffs_non_distress: u64,
    pub(super) separations_total: u64,
    pub(super) companies_founded: u64,
    pub(super) banks_founded: u64,
    pub(super) banks_closed: u64,
    pub(super) bank_runs_started: u64,
    pub(super) bank_resolutions_started: u64,
    pub(super) bankruptcies: u64,
    pub(super) wage_cuts: u64,
    pub(super) loans_issued: u64,
    pub(super) loans_ended: u64,
    pub(super) loans_defaulted: u64,
    pub(super) debt_distress_events: u64,
    pub(super) gov_transfers: u64,
    pub(super) stock_ipos: u64,
    pub(super) stock_delistings: u64,
    pub(super) stock_margin_calls: u64,
}

impl EventCounts {
    pub(super) fn accumulate(&mut self, event: &SimEvent) {
        match event {
            SimEvent::PopBirth { count, .. } => self.births += u64::from(*count),
            SimEvent::PopDeath { count, .. } => self.deaths += u64::from(*count),
            SimEvent::JobSwitched { .. } => self.job_switches += 1,
            SimEvent::CompanyLayoffs { laid_off_count, .. } => {
                self.layoffs_total += u64::from(*laid_off_count)
            }
            SimEvent::LaborChurnMonthly {
                quits,
                retirements,
                layoffs_non_distress,
                ..
            } => {
                self.separations_quits += u64::from(*quits);
                self.separations_retirements += u64::from(*retirements);
                self.separations_layoffs_non_distress += u64::from(*layoffs_non_distress);
                self.separations_total +=
                    u64::from(*quits) + u64::from(*retirements) + u64::from(*layoffs_non_distress);
            }
            SimEvent::CompanyFounded { .. } => self.companies_founded += 1,
            SimEvent::BankFounded { .. } => self.banks_founded += 1,
            SimEvent::BankClosed { .. } => self.banks_closed += 1,
            SimEvent::BankRunStarted { .. } => self.bank_runs_started += 1,
            SimEvent::BankResolutionStarted { .. } => self.bank_resolutions_started += 1,
            SimEvent::CompanyBankrupt { .. } => self.bankruptcies += 1,
            SimEvent::CompanyWageCut { .. } => self.wage_cuts += 1,
            SimEvent::LoanEmitted { .. } => self.loans_issued += 1,
            SimEvent::LoanEnded { .. } => self.loans_ended += 1,
            SimEvent::LoanWrittenOff { .. } => self.loans_defaulted += 1,
            SimEvent::DebtDistress { .. } => self.debt_distress_events += 1,
            SimEvent::GovTransfer { .. } => self.gov_transfers += 1,
            SimEvent::StockIpo { .. } => self.stock_ipos += 1,
            SimEvent::StockDelisted { .. } => self.stock_delistings += 1,
            SimEvent::StockMarginCall { .. } => self.stock_margin_calls += 1,
            _ => {}
        }
    }
}

pub(super) fn event_country_and_day(event: &SimEvent) -> (&str, u64) {
    match event {
        SimEvent::PopBirth {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::PopDeath {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LoanEmitted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LoanEnded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LoanWrittenOff {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::DebtDistress {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::GovTransfer {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::GovCompanyFunded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyFounded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankFounded {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankRunStarted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankRunWithdrawal {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankResolutionStarted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankResolved {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankClosed {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BankDepositHaircut {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyWageCut {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyLayoffs {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::LaborChurnMonthly {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::JobSwitched {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CompanyBankrupt {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockIpo {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockDividend {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockMarginCall {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockDelisted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockMarketClose {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::RatePolicyChange {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BondIssued {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BondDefault {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::CouponPaid {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::BondMatured {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::MoneyMinted {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::StockSecondaryOffering {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::ForeignDepositOpened {
            home_country_name,
            day,
            ..
        } => (home_country_name, *day),
        SimEvent::ForeignDepositClosed {
            home_country_name,
            day,
            ..
        } => (home_country_name, *day),
        SimEvent::DisasterApplied {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::HomePurchased {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::Foreclosure {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::MortgagePaidOff {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::HousingPriceChanged {
            country_name, day, ..
        } => (country_name, *day),
        SimEvent::RentUnpaid {
            country_name, day, ..
        } => (country_name, *day),
    }
}
