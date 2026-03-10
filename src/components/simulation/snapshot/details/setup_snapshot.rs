use super::*;

impl SetupSnapshot {
    pub(in crate::components::simulation) fn from_setup_form(
        setup: &crate::sim::setup::SetupForm,
        is_generating: bool,
    ) -> Self {
        Self {
            country_count_input: setup.country_count_input.clone(),
            thousands_per_country_input: setup.thousands_per_country_input.clone(),
            generation_seed_input: setup.generation_seed_input.clone(),
            active_field: setup.active_field,
            validation_error: setup.validation_error.clone(),
            memory_warning: setup.memory_warning.clone(),
            is_generating,
        }
    }
}
