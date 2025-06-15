use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use git2::Repository;
use tokio::fs::{create_dir, create_dir_all, write};

#[derive(clap::Args, Debug, Clone)]
pub struct Command {
    #[arg(value_name = "PATH", default_value = ".")]
    project_path: PathBuf,
}

impl Command {
    pub async fn execute(self) -> anyhow::Result<()> {
        if self.project_path.join("Maky.toml").exists() {
            Err(anyhow!(
                "`maky init` cannot be run on existing Maky packages"
            ))
        } else {
            create_dir_all(&self.project_path).await?;

            let project_path = self.project_path.canonicalize()?;

            if !Path::new(".git").exists() {
                Repository::init(&project_path)?;
            }

            create_dir(project_path.join("src")).await.ok();
            write(
                project_path.join(".gitignore"),
                include_str!("./assets/.gitignore"),
            )
            .await?;
            write(
                project_path.join("Maky.toml"),
                include_str!("./assets/Maky.toml").replace(
                    "{{name}}",
                    &project_path
                        .file_stem()
                        .map(OsStr::to_string_lossy)
                        .ok_or(anyhow!("can't create project name from directory"))?,
                ),
            )
            .await?;

            write(
                project_path.join("src/main.c"),
                include_str!("./assets/main.c"),
            )
            .await?;

            Ok(())
        }
    }
}
