mod build;
mod clean;
mod init;
mod run;

use std::path::{Path, PathBuf};

pub use build::*;
pub use clean::*;
pub use init::*;
pub use run::*;

pub fn add_mode_path(path: &Path, release: bool) -> PathBuf {
    path.join(if release {
        Path::new("release")
    } else {
        Path::new("debug")
    })
}

pub fn get_project_path(config_file: &str) -> (PathBuf, PathBuf) {
    let project_config_path = Path::new(config_file);

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
