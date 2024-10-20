use std::path::PathBuf;

use clap::Args;

use super::BuildArguments;

#[derive(Args, Debug)]
pub struct RunArguments {
    #[clap(flatten)]
    build: BuildArguments,

    /// Path of the source file to build and run
    file: PathBuf,

    /// Arguments for the source file to run
    args: Vec<String>,
}

pub fn run(arguments: RunArguments) -> anyhow::Result<()> {
    Ok(())
}
