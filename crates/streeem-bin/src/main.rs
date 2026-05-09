mod cli;
mod ratatui_renderer;
mod runtime;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let columns = cli.columns;
    let min_tile_width = cli.min_tile_width;
    let specs = cli.into_specs()?;
    runtime::run(specs, columns, min_tile_width).await
}
