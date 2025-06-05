use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use git2::Repository;
use tokio::fs::{create_dir, create_dir_all, write};

#[derive(clap::Args, Debug, Clone)]
pub struct Args {
    /// Folder to initialize
    #[arg(default_value = ".")]
    path: PathBuf,
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    if args.path.join("Maky.toml").exists() {
        Err(anyhow!(
            "`maky init` cannot be run on existing Maky packages"
        ))
    } else {
        create_dir_all(&args.path).await?;

        let path = args.path.canonicalize()?;

        if !Path::new(".git").exists() {
            Repository::init(&path)?;
        }

        create_dir(path.join("src")).await.ok();
        write(path.join(".gitignore"), include_str!("./assets/.gitignore")).await?;
        write(
            path.join("Maky.toml"),
            include_str!("./assets/Maky.toml").replace(
                "{{name}}",
                &path
                    .file_stem()
                    .map(OsStr::to_string_lossy)
                    .ok_or(anyhow!("can't create project name from directory"))?,
            ),
        )
        .await?;

        write(path.join("src/main.c"), include_str!("./assets/main.c")).await?;

        Ok(())
    }
}
