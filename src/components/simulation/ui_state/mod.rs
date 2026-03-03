use crate::sim::{UiCommand, company::Company, person::Person, world::World};

mod commands;
mod navigation;
mod types;
mod views;

pub use types::{
    COMPANY_PAGE_SIZE, COUNTRY_PAGE_SIZE, ChartScope, DetailsView, EventsSort, PEOPLE_PAGE_SIZE,
    RunningTab, SelectionFocus, UiState,
};
