use clap::Parser;
use cli::Cli;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod csv_export;
mod errors;
mod logging;
mod seed;
mod sim;
mod tui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::errors::init()?;
    crate::logging::init()?;

    let args = Cli::parse();
    let random_seed = seed::random_generation_seed();
    let env_seed = std::env::var(seed::GENERATION_SEED_ENV_VAR).ok();
    let generation_seed =
        seed::resolve_generation_seed(args.seed.as_deref(), env_seed.as_deref(), random_seed)
            .map_err(|message| color_eyre::eyre::eyre!(message))?;

    let mut app = App::new(
        args.tick_rate,
        args.frame_rate,
        args.sim_mode,
        args.fast_ui_hz,
        generation_seed,
        args.export_csv,
    )?;
    app.run().await?;
    Ok(())
}
