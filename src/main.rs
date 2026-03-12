use anyhow::Result;
use clap::Parser;
use yapi::app::App;
use yapi::cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    App::new(None)?.run(cli)
}
