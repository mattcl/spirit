use anyhow::Result;

mod cli;
mod settings;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::run().await
}
