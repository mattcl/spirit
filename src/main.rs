use anyhow::Result;

mod cli;
mod settings;

fn main() -> Result<()> {
    cli::Cli::run()
}
