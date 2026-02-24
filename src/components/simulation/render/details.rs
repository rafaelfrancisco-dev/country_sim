use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::{Simulation, theme};
use crate::sim::{job::job_by_id, names::person_name_from_seed};

impl Simulation {
    pub(super) fn draw_person_details(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let Some(country) = world.selected_country() else {
            frame.render_widget(
                Paragraph::new("No country is currently selected.")
                    .style(theme::primary_text())
                    .block(
                        Block::default()
                            .title(" Person Details ")
                            .borders(Borders::ALL)
                            .border_style(theme::neutral_border())
                            .title_style(theme::primary_text()),
                    ),
                area,
            );
            return;
        };

        let Some(person) = world.selected_person_on_page() else {
            frame.render_widget(
                Paragraph::new(
                    "No person available for the current selection. Press [Esc] to return.",
                )
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Person Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
                area,
            );
            return;
        };

        let name_seed = (u64::from(country.id) << 32) | u64::from(person.name_seed);
        let name = person_name_from_seed(name_seed);
        let job_label = person
            .job_type_id()
            .and_then(job_by_id)
            .map_or("Unemployed".to_string(), |job| job.name.to_string());
        let employer_label = person
            .employer_company_id()
            .and_then(|company_id| country.company_name(company_id))
            .unwrap_or("N/A")
            .to_string();
        let employment_label = if person.is_employed() {
            "Employed"
        } else {
            "Unemployed"
        };
        let details = vec![
            ratatui::text::Line::from(format!("Name: {}", name)),
            ratatui::text::Line::from(format!("Citizen ID: {}", person.id)),
            ratatui::text::Line::from(format!("Country: {}", country.name)),
            ratatui::text::Line::from(format!("Day: {}", world.day)),
            ratatui::text::Line::from(format!("Age: {}", person.age_years())),
            ratatui::text::Line::from(format!(
                "Education Track: {}",
                person.education_track.label()
            )),
            ratatui::text::Line::from(format!("Habilitation: {}", person.habilitation.label())),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(format!("Employment: {}", employment_label)),
            ratatui::text::Line::from(format!("Job: {}", job_label)),
            ratatui::text::Line::from(format!("Employer: {}", employer_label)),
            ratatui::text::Line::from(format!(
                "Monthly Salary: {}",
                Self::money(person.monthly_salary())
            )),
            ratatui::text::Line::from(format!(
                "Cash Balance: {}",
                Self::money(person.cash_balance)
            )),
            ratatui::text::Line::from(format!(
                "Loan Debt: {}",
                Self::money(person.loan.as_ref().map_or(0.0, |loan| loan.debt_balance))
            )),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(format!(
                "Last Gross Pay: {}",
                Self::money(person.last_month_gross_pay)
            )),
            ratatui::text::Line::from(format!(
                "Last Tax Paid: {}",
                Self::money(person.last_month_tax_paid)
            )),
            ratatui::text::Line::from(format!(
                "Last Net Pay: {}",
                Self::money(person.last_month_net_pay)
            )),
            ratatui::text::Line::from(format!(
                "Last Spending: {}",
                Self::money(person.last_month_spending)
            )),
            ratatui::text::Line::from(format!(
                "Last Loan Payment: {}",
                Self::money(person.last_month_loan_payment)
            )),
            ratatui::text::Line::from(format!(
                "Last New Loan: {}",
                Self::money(person.last_month_new_loan)
            )),
        ];

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Person Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }

    pub(super) fn draw_country_details(&self, frame: &mut Frame, area: Rect) {
        let Some(world) = self.engine.world() else {
            return;
        };

        let Some(country) = world.selected_country() else {
            frame.render_widget(
                Paragraph::new("No country is currently selected.")
                    .style(theme::primary_text())
                    .block(
                        Block::default()
                            .title(" Country Details ")
                            .borders(Borders::ALL)
                            .border_style(theme::neutral_border())
                            .title_style(theme::primary_text()),
                    ),
                area,
            );
            return;
        };

        let details = vec![
            ratatui::text::Line::from(format!("Name: {}", country.name)),
            ratatui::text::Line::from(format!("Country ID: {}", country.id)),
            ratatui::text::Line::from(format!("Day: {}", world.day)),
            ratatui::text::Line::from(format!("Month: {}", country.month)),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(format!("Population: {}", country.population())),
            ratatui::text::Line::from(format!(
                "Household Cash: {}",
                Self::money(country.household_cash_total)
            )),
            ratatui::text::Line::from(format!(
                "Company Cash: {}",
                Self::money(country.company_cash_total)
            )),
            ratatui::text::Line::from(format!(
                "Treasury: {}",
                Self::money(country.treasury_balance)
            )),
            ratatui::text::Line::from(format!(
                "Bank Reserve: {}",
                Self::money(country.bank.reserve_balance)
            )),
            ratatui::text::Line::from(format!(
                "Loans Outstanding: {}",
                Self::money(country.bank.created_money_outstanding)
            )),
            ratatui::text::Line::from(format!(
                "Money Supply: {}",
                Self::money(country.total_money_supply)
            )),
            ratatui::text::Line::from(format!(
                "Expected Supply: {}",
                Self::money(country.expected_money_supply)
            )),
            ratatui::text::Line::from(format!(
                "Supply Gap: {}",
                Self::money(country.money_supply_gap)
            )),
            ratatui::text::Line::from(format!(
                "Last Monthly Tax: {}",
                Self::money(country.monthly_tax_collected)
            )),
            ratatui::text::Line::from(format!(
                "Last Monthly Spending: {}",
                Self::money(country.monthly_household_spending)
            )),
            ratatui::text::Line::from(format!(
                "People Page: {}/{}",
                world.people_page + 1,
                world.total_people_pages()
            )),
        ];

        frame.render_widget(
            Paragraph::new(details)
                .style(theme::primary_text())
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .title(" Country Details ")
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }
}
