use super::Simulation;

impl Simulation {
    pub(super) fn reconcile_monthly_processing(&mut self) {}

    #[cfg(test)]
    pub(super) fn should_pause_daily_ticks_for_monthly_catchup(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::GameCommand;

    #[test]
    fn monthly_catchup_gate_is_disabled_for_synchronous_monthly_processing() {
        let simulation = Simulation::new();
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupNextField);
        simulation.apply_command_and_wait(GameCommand::SetupEditChar('1'));
        simulation.apply_command_and_wait(GameCommand::SetupSubmit);

        assert!(!simulation.should_pause_daily_ticks_for_monthly_catchup());
    }
}
