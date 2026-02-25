pub const EVENT_LOG_LIMIT: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    Person,
    Company,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimEvent {
    PopBirth { country_name: String, count: u32, day: u64 },
    PopDeath { country_name: String, count: u32, day: u64 },
    LoanEmitted { country_name: String, entity_kind: EntityKind, entity_id: u32, principal: f64, day: u64 },
    LoanEnded { country_name: String, entity_kind: EntityKind, entity_id: u32, day: u64 },
    LoanWrittenOff { country_name: String, entity_id: u32, principal_lost: f64, day: u64 },
    DebtDistress { country_name: String, entity_kind: EntityKind, entity_id: u32, unpaid_amount: f64, day: u64 },
    GovTransfer { country_name: String, total_recipients: u32, total_amount: f64, day: u64 },
}

impl SimEvent {
    pub fn format(&self) -> String {
        match self {
            Self::PopBirth { country_name, count, day } => {
                format!("[BIRTH] Day {day:>6} | {country_name}: +{count} birth(s)")
            }
            Self::PopDeath { country_name, count, day } => {
                format!("[DEATH] Day {day:>6} | {country_name}: -{count} death(s)")
            }
            Self::LoanEmitted { country_name, entity_kind, entity_id, principal, day } => {
                let kind = entity_kind_label(entity_kind);
                format!("[LOAN+] Day {day:>6} | {country_name}: {kind} #{entity_id} +${principal:.2} loan")
            }
            Self::LoanEnded { country_name, entity_kind, entity_id, day } => {
                let kind = entity_kind_label(entity_kind);
                format!("[LOAN=] Day {day:>6} | {country_name}: {kind} #{entity_id} loan paid off")
            }
            Self::LoanWrittenOff { country_name, entity_id, principal_lost, day } => {
                format!("[LOSS]  Day {day:>6} | {country_name}: Person #{entity_id} loan written off ${principal_lost:.2}")
            }
            Self::DebtDistress { country_name, entity_kind, entity_id, unpaid_amount, day } => {
                let kind = entity_kind_label(entity_kind);
                format!("[DEBT!] Day {day:>6} | {country_name}: {kind} #{entity_id} missed payment ${unpaid_amount:.2}")
            }
            Self::GovTransfer { country_name, total_recipients, total_amount, day } => {
                format!("[GOV]   Day {day:>6} | {country_name}: {total_recipients} recipients +${total_amount:.2} benefits")
            }
        }
    }
}

fn entity_kind_label(kind: &EntityKind) -> &'static str {
    match kind {
        EntityKind::Person => "Person",
        EntityKind::Company => "Company",
    }
}
