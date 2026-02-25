use clap::Parser;
use cli::Cli;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod errors;
mod logging;
mod sim;
mod tui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let mut app = App::new(
        args.tick_rate,
        args.frame_rate,
        args.sim_mode,
        args.fast_ui_hz,
    )?;
    app.run().await?;
    Ok(())
}
