use std::path::PathBuf;

pub mod build;
pub mod clean;
pub mod init;
pub mod run;

const TARGET_SELECTION: &str = "Target Selection";
const COMPILATION_OPTIONS: &str = "Compilation Options";
const MANIFEST_OPTIONS: &str = "Manifest Options";

#[derive(clap::Args, Debug, Clone)]
pub struct ProjectArgs {
    /// Path to Maky.toml
    #[arg(long="manifest-path", value_name="PATH", help_heading = MANIFEST_OPTIONS)]
    manifest: Option<PathBuf>,
}
