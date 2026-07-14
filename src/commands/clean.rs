use std::{
    fs::Metadata,
    io::{Write, stderr},
    path::PathBuf,
};

use async_recursion::async_recursion;
use async_walkdir::WalkDir;
use crossterm::{
    execute,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
};
use futures_lite::StreamExt;
use prettier_bytes::{ByteFormatter, Standard, Unit};
use tokio::{
    fs::{remove_dir_all, remove_file},
    spawn,
    sync::mpsc::{UnboundedSender, unbounded_channel},
};

use crate::{
    commands::{self, COMPILATION_OPTIONS},
    config::{self, Dependency, TypedDependency},
    helpers::{self, PathTarget},
    print_warning,
};

#[derive(clap::Args, Debug, Clone)]
pub struct Command {
    /// Display what would be deleted without deleting anything
    #[arg(short = 'n', long = "dry-run")]
    dry_run: bool,

    /// Whether or not to clean release artifacts
    #[arg(short, long, help_heading = COMPILATION_OPTIONS)]
    release: bool,

    #[command(flatten)]
    project: commands::ProjectArgs,
}

impl Command {
    pub async fn execute(self) -> anyhow::Result<()> {
        let project_paths = helpers::paths(
            self.project.manifest.as_ref().map(PathBuf::as_path),
            self.release,
        )?;
        let project_config = project_paths.config().await?;
        let package_config = project_config.package(&project_paths.project_path)?;
        let (metadata_sender, mut metadata_receiver) = unbounded_channel();

        let handle = spawn(self.clone().clean_package(
            project_config,
            project_paths,
            package_config,
            metadata_sender,
        ));

        let mut file_count = 0;
        let mut total_size = 0;
        let formatter = ByteFormatter::new()
            .standard(Standard::Binary)
            .unit(Unit::Bytes)
            .space(true);

        while let Some(metadata) = metadata_receiver.recv().await {
            file_count += 1;
            total_size += metadata.len();

            execute!(
                stderr(),
                Clear(ClearType::CurrentLine),
                Print("\r"),
                Print("     Removed ".dark_green().bold()),
                Print(format!(
                    "{file_count} files {}",
                    if total_size > 0 {
                        formatter.format(total_size).to_string()
                    } else {
                        String::new()
                    }
                )),
            )?;
            stderr().flush()?;
        }

        execute!(
            stderr(),
            Clear(ClearType::CurrentLine),
            Print("\r"),
            Print("     Removed ".dark_green().bold()),
            Print(format!(
                "{file_count} files {}",
                if total_size > 0 {
                    formatter.format(total_size).to_string()
                } else {
                    String::new()
                }
            )),
            Print("\n")
        )?;

        handle.await??;

        if self.dry_run {
            print_warning("no files deleted due to --dry-run")?;
        }

        Ok(())
    }

    #[async_recursion]
    async fn clean_package(
        self,
        project_config: config::Project,
        project_paths: helpers::ProjectPaths,
        package_config: config::Package,
        metadata_sender: UnboundedSender<Metadata>,
    ) -> anyhow::Result<()> {
        let directories = if self.release {
            vec![
                project_paths.maky_release_path.clone(),
                project_paths.maky_hash_path.clone(),
                project_paths
                    .project_path
                    .join(package_config.objects().target_release(true)),
                project_paths
                    .project_path
                    .join(package_config.binaries().target_release(true)),
            ]
        } else {
            vec![
                project_paths.maky_path.clone(),
                project_paths.project_path.join(package_config.objects()),
                project_paths.project_path.join(package_config.binaries()),
            ]
        };

        for (name, dependency_targets) in project_config.dependencies() {
            for dependency_target in dependency_targets {
                let Dependency {
                    dependency: TypedDependency::Maky { path, .. },
                    ..
                } = dependency_target
                else {
                    continue;
                };

                let (project_path, _) = path.path(&project_paths, &name)?;

                if project_path.exists() {
                    let project_paths = helpers::paths(Some(&project_path), self.release)?;
                    let project_config = project_paths.config().await?;
                    let package_config = project_config.package(&project_paths.project_path)?;

                    self.clone()
                        .clean_package(
                            project_config,
                            project_paths,
                            package_config,
                            metadata_sender.clone(),
                        )
                        .await?;
                }
            }
        }

        for directory in directories {
            if !directory.is_dir() {
                continue;
            }

            let mut entries = WalkDir::new(&directory);

            while let Some(entry) = entries.try_next().await? {
                let path = entry.path();
                let metadata = entry.metadata().await?;

                if !metadata.is_dir() {
                    if !self.dry_run && path.exists() {
                        remove_file(path).await?;
                    }

                    metadata_sender.send(metadata)?;
                }
            }

            if !self.dry_run && directory.exists() {
                remove_dir_all(directory).await?;
            }
        }

        Ok(())
    }
}
