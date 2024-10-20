mod build;
mod clean;
mod format;
mod init;
mod run;

use std::path::{Path, PathBuf};

pub use build::*;
pub use clean::*;
pub use format::*;
pub use init::*;
pub use run::*;

pub fn add_build_mode(path: &Path, release: bool) -> PathBuf {
    path.join(if release { "release" } else { "debug" })
}

pub fn get_project_config_path(project_config_path: &Path) -> (PathBuf, PathBuf) {
    if project_config_path.is_dir() {
        return (
            project_config_path.to_path_buf(),
            project_config_path.join("Maky.toml"),
        );
    }

    (
        project_config_path
            .parent()
            .unwrap_or(Path::new("./"))
            .to_path_buf(),
        project_config_path.to_path_buf(),
    )
}

#[derive(clap::Args, Debug)]
pub struct GlobalArguments {
    /// Maky config file or folder
    #[arg(short = 'f', long = "file", default_value = "./Maky.toml")]
    config_file: PathBuf,
}
