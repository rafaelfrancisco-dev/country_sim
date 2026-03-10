use super::*;

pub(in crate::components::simulation) fn country_policy_lines(
    world: &World,
    ui: &UiState,
) -> Vec<String> {
    let Some(country) = ui.selected_country(world) else {
        return vec!["No country is currently selected.".to_string()];
    };
    let currency_symbol = normalized_currency_symbol(&ui.currency_symbol);

    let form = &ui.country_policy_form;
    let fields = [
        (
            CountryPolicyField::TaxBracket1Max,
            "Tax Bracket 1 Max",
            country.sim_params.tax.bracket_1_max,
            false,
            true,
        ),
        (
            CountryPolicyField::TaxBracket2Max,
            "Tax Bracket 2 Max",
            country.sim_params.tax.bracket_2_max,
            false,
            true,
        ),
        (
            CountryPolicyField::TaxRate1,
            "Tax Rate 1",
            country.sim_params.tax.rate_1,
            true,
            false,
        ),
        (
            CountryPolicyField::TaxRate2,
            "Tax Rate 2",
            country.sim_params.tax.rate_2,
            true,
            false,
        ),
        (
            CountryPolicyField::TaxRate3,
            "Tax Rate 3",
            country.sim_params.tax.rate_3,
            true,
            false,
        ),
        (
            CountryPolicyField::UnemploymentBenefitRate,
            "Unemployment Benefit Rate",
            country.sim_params.fiscal.unemployment_benefit_rate,
            true,
            false,
        ),
        (
            CountryPolicyField::MedianIncomeReference,
            "Median Income Reference",
            country.sim_params.fiscal.median_income_reference,
            false,
            true,
        ),
        (
            CountryPolicyField::BasicNeedsCashThreshold,
            "Basic Needs Cash Threshold",
            country.sim_params.fiscal.basic_needs_cash_threshold,
            false,
            true,
        ),
        (
            CountryPolicyField::TargetInflationRate,
            "Target Inflation Rate",
            country.sim_params.monetary.target_inflation_rate,
            true,
            false,
        ),
        (
            CountryPolicyField::NeutralPolicyRate,
            "Neutral Policy Rate",
            country.sim_params.monetary.neutral_policy_rate,
            true,
            false,
        ),
        (
            CountryPolicyField::InflationCoefficient,
            "Inflation Coefficient",
            country.sim_params.monetary.inflation_coefficient,
            false,
            false,
        ),
        (
            CountryPolicyField::UnemploymentCoefficient,
            "Unemployment Coefficient",
            country.sim_params.monetary.unemployment_coefficient,
            false,
            false,
        ),
        (
            CountryPolicyField::NaturalUnemploymentRate,
            "Natural Unemployment Rate",
            country.sim_params.monetary.natural_unemployment_rate,
            true,
            false,
        ),
        (
            CountryPolicyField::RateFloor,
            "Rate Floor",
            country.sim_params.monetary.rate_floor,
            true,
            false,
        ),
        (
            CountryPolicyField::RateCeiling,
            "Rate Ceiling",
            country.sim_params.monetary.rate_ceiling,
            true,
            false,
        ),
        (
            CountryPolicyField::RateChangeEventThreshold,
            "Rate Change Event Threshold",
            country.sim_params.monetary.rate_change_event_threshold,
            true,
            false,
        ),
        (
            CountryPolicyField::PolicyRateStep,
            "Policy Rate Step",
            country.sim_params.monetary.policy_rate_step,
            true,
            false,
        ),
        (
            CountryPolicyField::MintInflationGapWeight,
            "Mint Inflation Gap Weight",
            country.sim_params.monetary.mint_inflation_gap_weight,
            true,
            false,
        ),
        (
            CountryPolicyField::MintUnemploymentGapWeight,
            "Mint Unemployment Gap Weight",
            country.sim_params.monetary.mint_unemployment_gap_weight,
            true,
            false,
        ),
        (
            CountryPolicyField::MintMaxFractionExpectedSupply,
            "Mint Max Fraction Expected Supply",
            country
                .sim_params
                .monetary
                .mint_max_fraction_expected_supply,
            true,
            false,
        ),
    ];

    let mut lines = vec![
        format!("Country: {}", country.name),
        format!("Country ID: {}", country.id),
        String::new(),
        "Editable Policy Overrides".to_string(),
        "Changes apply immediately and affect the next monthly cycle.".to_string(),
        String::new(),
    ];

    for (field, label, value, is_rate, is_money) in fields {
        let focused = form.active_field == field;
        let pointer = if focused { ">" } else { " " };
        let formatted_value = if is_rate {
            format!("{value:.4}")
        } else if is_money {
            money(value, currency_symbol)
        } else {
            format!("{value:.2}")
        };
        let mut line = format!("{pointer} {label}: {formatted_value}");

        if focused && form.is_editing {
            line.push_str(&format!("  | draft: {}", form.draft_input));
        }

        lines.push(line);
    }

    lines.push(String::new());
    if form.is_editing {
        lines.push("Editing: type value, [Enter] save, [Esc] cancel.".to_string());
    } else {
        lines.push(
            "Navigate: [w/s] field, [Enter] edit, [r] reset managed defaults, [Esc] back."
                .to_string(),
        );
    }
    if let Some(error) = &form.validation_error {
        lines.push(format!("Validation: {error}"));
    }

    lines
}
