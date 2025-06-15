use std::path::{Path, PathBuf};

use anyhow::anyhow;
use tokio::fs::read_to_string;

use crate::{config, helpers::PathTarget};

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

impl ProjectArgs {
    pub fn paths(&self, release: bool) -> anyhow::Result<ProjectPaths> {
        let manifest_file_arg = self
            .manifest
            .as_ref()
            .map(PathBuf::as_path)
            .unwrap_or(Path::new("Maky.toml"));
        let manifest_file = if manifest_file_arg.is_dir() {
            manifest_file_arg.join("Maky.toml")
        } else {
            manifest_file_arg.to_path_buf()
        };

        if !manifest_file.is_file() {
            return Err(anyhow!("`{}` is not a file", manifest_file_arg.display()));
        }

        let Some(project_path) = manifest_file.parent().map(Path::to_path_buf) else {
            return Err(anyhow!(
                "can't find parent directory of `{}`",
                manifest_file.display()
            ));
        };
        let maky_path = project_path.join(".maky").target_release(release);
        let maky_hash_path = maky_path.join("hash");
        let maky_includes_path = maky_path.join("include");

        Ok(ProjectPaths {
            manifest_file,
            project_path,
            maky_path,
            maky_hash_path,
            maky_includes_path,
        })
    }
}

pub struct ProjectPaths {
    manifest_file: PathBuf,
    project_path: PathBuf,
    maky_path: PathBuf,
    maky_hash_path: PathBuf,
    maky_includes_path: PathBuf,
}

impl ProjectPaths {
    pub async fn config(&self) -> anyhow::Result<config::Project> {
        let file = read_to_string(&self.manifest_file).await?;

        Ok(toml::from_str(&file)?)
    }
}
