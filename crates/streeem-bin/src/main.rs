mod cli;
mod debug_log;
mod input_bytes;
mod ratatui_renderer;
mod runtime;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    debug_log::init();
    let cli = cli::Cli::parse();
    let columns = cli.columns;
    let min_tile_width = cli.min_tile_width;
    let specs = cli.into_specs()?;
    debug_log::log(&format!(
        "main: parsed {n} command specs, columns={columns:?}, min_tile_width={min_tile_width:?}",
        n = specs.len()
    ));
    runtime::run(specs, columns, min_tile_width).await
}
