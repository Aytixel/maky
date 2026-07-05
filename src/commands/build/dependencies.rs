mod maky;
mod pkg;

use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use anyhow::anyhow;
use tokio::fs::remove_dir_all;

use crate::{
    commands::build::dependencies::{maky::get_maky_dependency, pkg::get_pkg_dependency},
    config, helpers, print_warning,
};

static PKG_CONFIG_COMMAND: LazyLock<which::Result<PathBuf>> = LazyLock::new(|| {
    which::which("pkg-config")
        .or_else(|_| which::which("pkgconf"))
        .or_else(|_| which::which("pkg-config-lite"))
        .or_else(|_| which::which("pkgconf-lite"))
});

#[derive(Debug)]
pub struct DependencyProject {
    pub config: config::Project,
    pub paths: helpers::ProjectPaths,
    pub package: config::Package,
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Default)]
pub struct Dependency {
    // Dependency project to compile before
    pub project: Option<DependencyProject>,

    // Compile
    pub includes: Vec<String>,
    pub defines: Vec<String>,
    pub cflags: Vec<String>,

    // Link
    pub directories: Vec<String>,
    pub libraries: Vec<String>,
    pub lflags: Vec<String>,
}

impl Dependency {
    pub async fn get_dependencies(
        project_config: &config::Project,
        project_paths: &helpers::ProjectPaths,
        release: bool,
        update: bool,
    ) -> anyhow::Result<HashMap<String, Self>> {
        if project_paths.maky_includes_path.is_dir() {
            remove_dir_all(&project_paths.maky_includes_path).await?;
        }

        let mut dependencies = HashMap::new();
        let mut skip_pkg_config = false;

        for (name, dependency_targets) in project_config.dependencies() {
            for dependency_target in dependency_targets {
                match dependency_target.dependency {
                    config::TypedDependency::Pkg { paths, pkg } => {
                        if skip_pkg_config {
                            continue;
                        }

                        let Ok(pkg_config_command) = PKG_CONFIG_COMMAND.as_ref() else {
                            print_warning(anyhow!(
                                "can't find `pkg-config`, using fallback dependency instead"
                            ))?;
                            skip_pkg_config = true;
                            continue;
                        };

                        match get_pkg_dependency(pkg_config_command, paths, pkg).await {
                            Ok(mut dependency) => {
                                dependency.defines.extend(dependency_target.defines);
                                dependency.cflags.extend(dependency_target.cflags);
                                dependency.lflags.extend(dependency_target.lflags);

                                dependencies.insert(name, dependency);
                                break;
                            }
                            Err(error) => {
                                print_warning(anyhow!(
                                    "pkg-config error `\n{error}`, using `{name}` fallback dependency instead"
                                ))?;
                                continue;
                            }
                        }
                    }
                    config::TypedDependency::Maky {
                        version,
                        path,
                        libraries,
                    } => {
                        match get_maky_dependency(
                            &name,
                            project_paths,
                            release,
                            version,
                            path,
                            libraries,
                            update,
                        )
                        .await
                        {
                            Ok(mut dependency) => {
                                dependency.defines.extend(dependency_target.defines);
                                dependency.cflags.extend(dependency_target.cflags);
                                dependency.lflags.extend(dependency_target.lflags);

                                dependencies.insert(name, dependency);
                                break;
                            }
                            Err(error) => {
                                print_warning(anyhow!(
                                    "{error}, using `{name}` fallback dependency instead"
                                ))?;
                                continue;
                            }
                        }
                    }
                    config::TypedDependency::Local {
                        includes,
                        directories,
                        libraries,
                    } => {
                        dependencies.insert(
                            name,
                            Self {
                                project: None,
                                includes,
                                defines: dependency_target.defines,
                                cflags: dependency_target.cflags,
                                directories,
                                libraries,
                                lflags: dependency_target.lflags,
                            },
                        );
                        break;
                    }
                }
            }
        }

        Ok(dependencies)
    }
}
