use crate::sim::{UiCommand, company::Company, person::Person, world::World};

mod types;
mod commands;
mod navigation;
mod views;

pub use types::{
    ChartScope, DetailsView, EventsSort, RunningTab, SelectionFocus, UiState, COMPANY_PAGE_SIZE,
    COUNTRY_PAGE_SIZE, PEOPLE_PAGE_SIZE,
};
