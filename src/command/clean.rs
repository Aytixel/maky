use clap::Args;

use super::GlobalArguments;

#[derive(Args, Debug)]
pub struct CleanArguments {
    #[clap(flatten)]
    global: GlobalArguments,
}

pub fn clean(arguments: CleanArguments) -> anyhow::Result<()> {
    Ok(())
}
