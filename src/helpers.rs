use std::{
    io,
    path::{Path, PathBuf, absolute},
};

use anyhow::anyhow;
use tokio::fs::{create_dir, create_dir_all, read_to_string, remove_dir_all};

use crate::config::{self, Package};

pub trait PathTarget {
    fn target_release(&self, release: bool) -> PathBuf;
}

impl PathTarget for Path {
    fn target_release(&self, release: bool) -> PathBuf {
        self.join(if release { "release" } else { "debug" })
    }
}

impl PathTarget for PathBuf {
    fn target_release(&self, release: bool) -> PathBuf {
        self.join(if release { "release" } else { "debug" })
    }
}

pub trait TryStripPrefixPath {
    fn try_strip_prefix<P: AsRef<Path>>(&self, parent: P) -> Self;
}

impl TryStripPrefixPath for &Path {
    fn try_strip_prefix<P: AsRef<Path>>(&self, parent: P) -> Self {
        self.strip_prefix(parent).unwrap_or(self)
    }
}

impl TryStripPrefixPath for PathBuf {
    fn try_strip_prefix<P: AsRef<Path>>(&self, parent: P) -> Self {
        self.strip_prefix(parent).unwrap_or(self).to_path_buf()
    }
}

pub trait AsStrVec {
    fn as_str_vec(&self) -> Vec<&str>;
}

impl AsStrVec for Vec<String> {
    fn as_str_vec(&self) -> Vec<&str> {
        self.iter().map(String::as_str).collect()
    }
}

pub fn paths(manifest: Option<&Path>, release: bool) -> anyhow::Result<ProjectPaths> {
    let manifest_file_arg = manifest.unwrap_or(Path::new("Maky.toml"));
    let manifest_file = absolute(if manifest_file_arg.is_dir() {
        manifest_file_arg.join("Maky.toml")
    } else {
        manifest_file_arg.to_path_buf()
    })?;

    if !manifest_file.is_file() {
        return Err(anyhow!("`{}` is not a file", manifest_file_arg.display()));
    }

    let Some(project_path) = manifest_file.parent().map(Path::to_path_buf) else {
        return Err(anyhow!(
            "can't find parent directory of `{}`",
            manifest_file.display()
        ));
    };
    let maky_path = project_path.join(".maky");
    let maky_release_path = maky_path.target_release(release);
    let maky_hash_path = maky_release_path.join("hash");
    let maky_includes_path = maky_path.join("include");
    let maky_dependencies_path = maky_path.join("deps");

    Ok(ProjectPaths {
        manifest_file,
        project_path,
        maky_path,
        maky_release_path,
        maky_hash_path,
        maky_includes_path,
        maky_dependencies_path,
    })
}

#[derive(Debug, Clone)]
pub struct ProjectPaths {
    pub manifest_file: PathBuf,
    pub project_path: PathBuf,
    pub maky_path: PathBuf,
    pub maky_release_path: PathBuf,
    pub maky_hash_path: PathBuf,
    pub maky_includes_path: PathBuf,
    pub maky_dependencies_path: PathBuf,
}

impl ProjectPaths {
    pub async fn config(&self) -> anyhow::Result<config::Project> {
        let file = read_to_string(&self.manifest_file).await?;

        Ok(toml::from_str(&file)?)
    }

    pub async fn init_directories(
        &self,
        package_config: &Package,
        release: bool,
    ) -> anyhow::Result<()> {
        if !self.maky_path.is_dir() {
            create_dir(&self.maky_path).await?;
        }

        if !self.maky_release_path.is_dir() {
            create_dir(&self.maky_release_path).await?;
        }

        let binaries_path = self.binaries_path(package_config, release);
        if !binaries_path.is_dir() {
            create_dir_all(&binaries_path).await?;
        }

        let objects_path = self.objects_path(package_config, release);
        if !objects_path.is_dir() {
            create_dir_all(&objects_path).await?;
        }

        Ok(())
    }

    pub fn binaries_path(&self, package_config: &Package, release: bool) -> PathBuf {
        self.project_path
            .join(&package_config.binaries().target_release(release))
    }

    pub fn objects_path(&self, package_config: &Package, release: bool) -> PathBuf {
        self.project_path
            .join(&package_config.objects().target_release(release))
    }

    pub fn include_paths(&self, package_config: &Package) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = package_config
            .includes
            .iter()
            .chain(package_config.sources.iter())
            .map(|path| self.project_path.join(path))
            .collect();

        paths.push(self.maky_includes_path.clone());

        paths
    }
}

pub async fn symlink(original: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    remove_dir_all(&link).await.ok();

    #[cfg(target_family = "windows")]
    if original.as_ref().is_file() {
        tokio::fs::symlink_file(original, link).await
    } else {
        tokio::fs::symlink_dir(original, link).await
    }
    #[cfg(target_family = "unix")]
    tokio::fs::symlink(original, link).await
}
