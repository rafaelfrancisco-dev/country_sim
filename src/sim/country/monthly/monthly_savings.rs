use super::*;

impl Country {
    pub(super) fn process_savings_interest(&mut self) {
        let deposit_rate = (self.policy_rate - self.sim_params.monetary.deposit_spread).max(0.0);
        if deposit_rate <= 0.0 {
            return;
        }
        let monthly_rate = deposit_rate / 12.0;
        let min_balance = self.sim_params.monetary.savings_min_balance;

        // Compute gross interest owed (read-only, parallelizable)
        let people_gross: f64 = self
            .people
            .par_chunks(PEOPLE_CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .filter(|p| p.cash_balance > min_balance)
                    .map(|p| round_cents(p.cash_balance * monthly_rate))
                    .sum::<f64>()
            })
            .sum();
        let companies_gross: f64 = self
            .companies
            .iter()
            .filter(|c| c.cash_balance > min_balance)
            .map(|c| round_cents(c.cash_balance * monthly_rate))
            .sum();

        let total_gross = round_cents(people_gross + companies_gross);
        if total_gross <= 0.0 {
            return;
        }

        // Cap at available bank reserves (never go negative)
        let available = self.bank.reserve_balance.max(0.0);
        let total_paid = round_cents(total_gross.min(available));
        if total_paid <= 0.0 {
            return;
        }
        let scale = total_paid / total_gross;

        self.bank.reserve_balance = round_cents(self.bank.reserve_balance - total_paid);

        // Pay people (sequential write-back)
        let mut total_people_paid = 0.0_f64;
        for person in &mut self.people {
            if person.cash_balance <= min_balance {
                continue;
            }
            let net = round_cents(round_cents(person.cash_balance * monthly_rate) * scale);
            if net > 0.0 {
                person.cash_balance = round_cents(person.cash_balance + net);
                person.last_month_savings_interest =
                    round_cents(person.last_month_savings_interest + net);
                total_people_paid = round_cents(total_people_paid + net);
            }
        }

        // Pay companies
        let mut total_companies_paid = 0.0_f64;
        for company in &mut self.companies {
            if company.cash_balance <= min_balance {
                continue;
            }
            let net = round_cents(round_cents(company.cash_balance * monthly_rate) * scale);
            if net > 0.0 {
                company.cash_balance = round_cents(company.cash_balance + net);
                company.last_month_savings_interest =
                    round_cents(company.last_month_savings_interest + net);
                total_companies_paid = round_cents(total_companies_paid + net);
            }
        }

        self.monthly_savings_interest_paid =
            round_cents(total_people_paid + total_companies_paid);
    }
}
