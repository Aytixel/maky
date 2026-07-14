use std::path::PathBuf;

use anyhow::anyhow;
use async_recursion::async_recursion;
use async_walkdir::WalkDir;
use futures_lite::StreamExt;
use semver::VersionReq;
use tokio::fs::{create_dir_all, read_dir};

use crate::{
    commands::build::dependencies::{Dependency, DependencyProject},
    config::MakyPathDependency,
    file::is_header_file,
    git::pull,
    helpers::{self, PathTarget, symlink},
};

#[async_recursion]
pub async fn get_maky_dependency(
    name: &str,
    parent_project_paths: &helpers::ProjectPaths,
    release: bool,
    version: Option<VersionReq>,
    path: MakyPathDependency,
    libraries: Vec<String>,
    update: bool,
) -> anyhow::Result<Dependency> {
    let (project_path, git) = path.path(&parent_project_paths, name)?;

    if let Some(git) = &git {
        pull(git, update)?;
    }

    let project_paths = helpers::paths(Some(project_path.as_path()), release)?;
    let project_config = project_paths.config().await?;
    let project_package = project_config.package(&project_paths.project_path)?;

    match (version, &project_package.version) {
        (Some(requirement), Some(version)) => {
            if !requirement.matches(version) {
                return Err(anyhow!(
                    "version `{version}` doesn't meet required version `{requirement}`"
                ));
            }
        }
        (Some(requirement), None) => {
            return Err(anyhow!(
                "version requirement found `{requirement}` but no version"
            ));
        }
        (None, Some(_)) | (None, None) => {}
    }

    let dependencies =
        Dependency::get_dependencies(&project_config, &project_paths, release, update).await?;
    let includes = vec![
        parent_project_paths
            .maky_includes_path
            .strip_prefix(&parent_project_paths.project_path)?
            .to_string_lossy()
            .to_string(),
    ];
    let directories = vec![
        project_paths
            .project_path
            .join(project_package.binaries().target_release(release))
            .to_string_lossy()
            .to_string(),
    ];

    symlink_header_files(
        project_package
            .includes
            .iter()
            .map(|include| project_paths.project_path.join(include))
            .collect(),
        parent_project_paths
            .maky_includes_path
            .join("deps")
            .join(name),
    )
    .await?;
    symlink_header_directories(
        project_paths.maky_includes_path.join("deps"),
        parent_project_paths.maky_includes_path.join("deps"),
    )
    .await?;

    Ok(Dependency {
        project: Some(DependencyProject {
            config: project_config,
            package: project_package,
            paths: project_paths,
            dependencies,
        }),
        includes,
        directories,
        libraries,
        git,
        ..Default::default()
    })
}

async fn symlink_header_files(
    input_directories: Vec<PathBuf>,
    output_directory: PathBuf,
) -> anyhow::Result<()> {
    for input in input_directories {
        if !input.exists() {
            continue;
        }

        let mut entries = WalkDir::new(&input);

        while let Some(entry) = entries.try_next().await? {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let Some(extension) = path.extension() else {
                continue;
            };

            if is_header_file(extension) {
                let new_path = output_directory.join(path.strip_prefix(&input)?);

                create_dir_all(new_path.parent().unwrap()).await?;
                symlink(path, new_path).await?;
            }
        }
    }

    Ok(())
}

async fn symlink_header_directories(
    input_directory: PathBuf,
    output_directory: PathBuf,
) -> anyhow::Result<()> {
    create_dir_all(&output_directory).await?;

    if input_directory.exists() {
        let mut directory_reader = read_dir(&input_directory).await?;

        while let Some(entry) = directory_reader.next_entry().await? {
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let new_path = output_directory.join(path.strip_prefix(&input_directory)?);

            symlink(path, new_path).await?;
        }
    }

    Ok(())
}
