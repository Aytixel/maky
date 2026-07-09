use std::{
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use async_recursion::async_recursion;
use async_walkdir::WalkDir;
use futures_lite::StreamExt;
use git2::Repository;
use semver::VersionReq;
use tokio::fs::{create_dir_all, read_dir};

use crate::{
    commands::build::dependencies::{Dependency, DependencyProject},
    config::MakyPathDependency,
    file::is_header_file,
    helpers::{self, PathTarget, symlink},
};

fn fetch_default_branch(
    repository: &Repository,
    project_path: &Path,
    update: bool,
) -> anyhow::Result<String> {
    let default_branch_path = project_path.join(".git/default_branch");

    if update || !default_branch_path.exists() {
        let mut remote = repository.find_remote("origin")?;

        remote.fetch(&[] as &[&str], None, None)?;

        let default_branch = remote
            .default_branch()?
            .as_str()
            .unwrap_or("main")
            .to_string();

        write(&default_branch_path, &default_branch)?;

        Ok(default_branch)
    } else {
        Ok(read_to_string(default_branch_path)?)
    }
}

fn pull(project_path: &Path, url: String, rev: Option<String>, update: bool) -> anyhow::Result<()> {
    let repository = if project_path.is_dir() {
        Repository::open(&project_path)?
    } else {
        Repository::clone_recurse(&url, &project_path)?
    };

    let default_branch = fetch_default_branch(&repository, &project_path, update)?;
    let rev = rev.unwrap_or(default_branch);

    let (object, reference) = repository.revparse_ext(&rev)?;

    repository.checkout_tree(&object, None)?;

    if let Some(mut reference) = reference {
        let fetch_head = if update {
            &repository.find_reference("FETCH_HEAD")?
        } else {
            &reference
        };
        let fetch_commit = repository.reference_to_annotated_commit(fetch_head)?;
        let analysis = repository.merge_analysis(&[&fetch_commit])?;

        if analysis.0.is_up_to_date() {
            repository.set_head(reference.name().unwrap())?;
        } else if analysis.0.is_fast_forward() {
            reference.set_target(fetch_commit.id(), "Fast-Forward")?;
            repository.set_head(reference.name().unwrap())?;
            repository.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else {
            return Err(anyhow!("git can't fast-forward"));
        }
    } else {
        repository.set_head_detached(object.id())?;
    }

    Ok(())
}

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
    let (project_path, git) = path.path(&parent_project_paths)?;

    if let Some((url, rev, path)) = git {
        pull(&path, url.to_string(), rev, update)?;
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
