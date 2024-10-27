use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    kdam::term::init(true);

    let args = Args::parse();

    if let Some(command) = args.command {
        match command {}
    }

    return Ok(());
}
