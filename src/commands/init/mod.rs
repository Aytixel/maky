use std::path::{Path, PathBuf};

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

        if !Path::new(".git").exists() {
            Repository::init(&args.path)?;
        }

        create_dir(args.path.join("src")).await.ok();
        write(
            args.path.join(".gitignore"),
            include_str!("./assets/.gitignore"),
        )
        .await?;
        write(
            args.path.join("Maky.toml"),
            include_str!("./assets/Maky.toml"),
        )
        .await?;

        write(
            args.path.join("src/main.c"),
            include_str!("./assets/main.c"),
        )
        .await?;

        Ok(())
    }
}
