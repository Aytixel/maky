use clap::{ArgAction, Args};

use super::GlobalArguments;

#[derive(Args, Debug)]
pub struct BuildArguments {
    #[clap(flatten)]
    global: GlobalArguments,

    /// Build in release mode
    #[arg(long)]
    release: bool,

    /// Rebuild every time
    #[arg(long)]
    rebuild: bool,

    /// Enable a pretty display
    #[arg(long, action=ArgAction::Set, default_value_t = true)]
    pretty: bool,
}

pub fn build(arguments: BuildArguments) -> anyhow::Result<bool> {
    Ok(true)
}
