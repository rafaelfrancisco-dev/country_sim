mod housing;
mod messages;
mod utils;

use super::SimEvent;

impl SimEvent {
    pub fn format(&self) -> String {
        self.format_with_currency(crate::sim::setup::DEFAULT_CURRENCY_SYMBOL)
    }

    pub fn format_with_currency(&self, currency_symbol: &str) -> String {
        messages::format_event(self, currency_symbol)
    }
}
