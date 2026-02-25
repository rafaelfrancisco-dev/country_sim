use clap::ValueEnum;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, ValueEnum)]
#[value(rename_all = "kebab_case")]
pub enum SimulationMode {
    #[default]
    Realtime,
    MaxSpeed,
}

impl SimulationMode {
    pub const REALTIME: u8 = 0;
    pub const MAX_SPEED: u8 = 1;

    pub fn as_atomic_u8(self) -> u8 {
        match self {
            Self::Realtime => Self::REALTIME,
            Self::MaxSpeed => Self::MAX_SPEED,
        }
    }

    pub fn from_atomic_u8(value: u8) -> Self {
        match value {
            Self::MAX_SPEED => Self::MaxSpeed,
            _ => Self::Realtime,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Realtime => "REALTIME",
            Self::MaxSpeed => "MAX",
        }
    }
}
