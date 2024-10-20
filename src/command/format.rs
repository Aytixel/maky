use clap::Args;

use super::GlobalArguments;

#[derive(Args, Debug)]
pub struct FormatArguments {
    #[clap(flatten)]
    global: GlobalArguments,

    /// Files to format
    files: Vec<String>,

    /// Tabulation size in spaces
    #[arg(short = 't', long = "tab", default_value = "4")]
    tab_size: usize,
}

pub async fn format(arguments: FormatArguments) -> anyhow::Result<()> {
    let tab = " ".repeat(arguments.tab_size).to_string();
    Ok(())
}
