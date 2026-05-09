mod cli;
mod runtime;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    let columns = cli.columns;
    let specs = cli.into_specs()?;
    runtime::run(specs, columns).await
}
