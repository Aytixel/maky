mod compile;
mod dependencies;
mod file;
mod link;

use std::{collections::HashMap, io::stderr, path::PathBuf};

use async_recursion::async_recursion;
use crossterm::{
    execute,
    style::{Print, Stylize},
};
use tokio::time::Instant;

pub use compile::compile;

use crate::{
    commands::{
        self, COMPILATION_OPTIONS, TARGET_SELECTION,
        build::link::{link, link_flags},
    },
    config::{self, Package, Target, TargetType},
    git::GitRepositoryInfo,
    helpers::{self, PathTarget, ProjectPaths, symlink},
};

pub use crate::commands::build::{
    dependencies::{Dependency, DependencyProject},
    file::{
        add_source_files_dependencies, add_uncompiled_source_files, check_updated_source_files,
        filter_source_files, get_source_files_reverse_dependencies, get_targets_source_files,
        scan_source_files,
    },
};

#[derive(clap::Args, Debug, Clone)]
pub struct Command {
    /// Build only this package's library
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub lib: bool,

    /// Build all binaries
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub bins: bool,

    /// Build only the specified binary
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub bin: Vec<String>,

    /// Build all examples
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub examples: bool,

    /// Build only the specified example
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub example: Vec<String>,

    /// Build all tests
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub tests: bool,

    /// Build only the specified test target
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub test: Vec<String>,

    /// Build all benches
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub benches: bool,

    /// Build only the specified bench target
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub bench: Vec<String>,

    /// Build all targets
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub all_targets: bool,

    /// Build artifacts in release mode, with optimizations
    #[arg(short, long, help_heading = COMPILATION_OPTIONS)]
    pub release: bool,

    /// Update dependencies
    #[arg(short, long, help_heading = COMPILATION_OPTIONS)]
    pub update: bool,

    #[command(flatten)]
    pub project: commands::ProjectArgs,
}

impl Command {
    pub async fn execute(self) -> anyhow::Result<()> {
        self.build().await?;
        Ok(())
    }

    pub async fn build(
        self,
    ) -> anyhow::Result<(
        config::Project,
        helpers::ProjectPaths,
        config::Package,
        Vec<Target>,
        Vec<String>,
    )> {
        let time = Instant::now();
        let project_paths = helpers::paths(
            self.project.manifest.as_ref().map(PathBuf::as_path),
            self.release,
        )?;
        let project_config = project_paths.config().await?;
        let package_config = project_config.package(&project_paths.project_path)?;
        let dependencies = Dependency::get_dependencies(
            &project_config,
            &project_paths,
            self.release,
            self.update,
        )
        .await?;

        let (targets, lflags) = self
            .build_package(
                &project_config,
                &project_paths,
                &package_config,
                &dependencies,
                None,
            )
            .await?;

        execute!(
            stderr(),
            Print("    Finished ".dark_green().bold()),
            Print("`"),
            Print(if self.release { "release" } else { "dev" }.reset()),
            Print("` profile ["),
            Print(if self.release {
                "optimized"
            } else {
                "unoptimized + debuginfo"
            }),
            Print("]"),
            Print(format!(" target(s) in {:.2?}\n", time.elapsed()))
        )?;

        Ok((
            project_config,
            project_paths,
            package_config,
            targets,
            lflags,
        ))
    }

    #[async_recursion]
    async fn build_package(
        &self,
        project_config: &config::Project,
        project_paths: &helpers::ProjectPaths,
        package_config: &config::Package,
        dependencies: &HashMap<String, Dependency>,
        git: Option<&GitRepositoryInfo>,
    ) -> anyhow::Result<(Vec<Target>, Vec<String>)> {
        project_paths
            .init_directories(&package_config, self.release)
            .await?;

        let targets = project_config.targets_selection(
            self.lib,
            self.bins,
            &self.bin,
            self.examples,
            &self.example,
            self.tests,
            &self.test,
            self.benches,
            &self.bench,
            self.all_targets,
        );

        // build maky dependencies first
        let mut dependencies_lflags = HashMap::new();

        for target in &targets {
            for import in &target.import {
                if let Some(dependency) = dependencies.get(import)
                    && let Some(project) = &dependency.project
                {
                    let mut dependency_command = self.clone();

                    dependency_command.all_targets = false;
                    dependency_command.bins = false;
                    dependency_command.bin = dependency.libraries.clone();
                    dependency_command.lib = false;
                    dependency_command.examples = false;
                    dependency_command.example = Vec::new();
                    dependency_command.tests = false;
                    dependency_command.test = Vec::new();
                    dependency_command.benches = false;
                    dependency_command.bench = Vec::new();

                    dependencies_lflags
                        .entry(target.name()?)
                        .or_insert(Vec::new())
                        .extend(
                            dependency_command
                                .build_package(
                                    &project.config,
                                    &project.paths,
                                    &project.package,
                                    &project.dependencies,
                                    dependency.git.as_ref(),
                                )
                                .await?
                                .1,
                        );

                    symlink_libraries(project, project_paths, package_config, self.release).await?;
                }
            }
        }

        let binaries_path = project_paths.binaries_path(package_config, self.release);
        let objects_path = project_paths.objects_path(package_config, self.release);
        let include_paths = project_paths.include_paths(package_config);

        // process source and include files
        let source_files =
            scan_source_files(&targets, project_paths, package_config, &include_paths).await?;
        let source_files_reverse_dependencies =
            get_source_files_reverse_dependencies(&source_files);

        let updated_source_files = check_updated_source_files(project_paths, &source_files).await?;
        let updated_source_files =
            add_uncompiled_source_files(&objects_path, &source_files, updated_source_files);
        let updated_source_files = add_source_files_dependencies(
            &source_files_reverse_dependencies,
            updated_source_files,
        )?;
        let updated_source_files = filter_source_files(updated_source_files);

        if !updated_source_files.is_empty() {
            print_compile(project_paths, package_config, git)?;

            compile(
                project_paths,
                package_config,
                dependencies,
                &updated_source_files,
                &source_files,
                self.release,
                &[],
            )
            .await?;
        }

        let mut targets_source_files =
            get_targets_source_files(&targets, &source_files, &source_files_reverse_dependencies)
                .await?;
        let mut static_lflags = Vec::new();
        let targets_lflags: HashMap<String, Vec<String>> = targets_source_files
            .iter()
            .map(|(target, _)| {
                let lflags =
                    link_flags(package_config, dependencies, target, &dependencies_lflags)?;

                if target.target_type == TargetType::StaticLib {
                    static_lflags.extend(lflags.clone());
                }

                Ok((target.name()?, lflags))
            })
            .collect::<anyhow::Result<_>>()?;

        targets_source_files.retain(|target, source_files| {
            !source_files.is_disjoint(&updated_source_files)
                || (target.binary_name().map_or(false, |binary_name| {
                    !binaries_path.join(binary_name).exists()
                }))
        });

        if !targets_source_files.is_empty() {
            if updated_source_files.is_empty() {
                print_compile(project_paths, package_config, git)?;
            }

            link(
                project_paths,
                package_config,
                &targets_source_files,
                &source_files,
                &targets_lflags,
                self.release,
            )
            .await?;
        }

        Ok((targets, static_lflags))
    }
}

#[async_recursion]
async fn symlink_libraries(
    project: &DependencyProject,
    project_paths: &ProjectPaths,
    package_config: &Package,
    release: bool,
) -> anyhow::Result<()> {
    for dependency in project.dependencies.values() {
        if let Some(project) = &dependency.project {
            symlink_libraries(project, project_paths, package_config, release).await?;
        }
    }

    for binary in project.config.binaries() {
        if binary.target_type == TargetType::Dylib {
            let binary_name = binary.binary_name()?;
            let source_library_path = project.paths.project_path.join(
                project
                    .package
                    .binaries()
                    .target_release(release)
                    .join(&binary_name),
            );
            let target_library_path = project_paths.project_path.join(
                package_config
                    .binaries()
                    .target_release(release)
                    .join(binary_name),
            );

            if source_library_path.exists() {
                symlink(source_library_path, target_library_path).await?;
            }
        }
    }

    Ok(())
}

fn print_compile(
    project_paths: &ProjectPaths,
    package_config: &Package,
    git: Option<&GitRepositoryInfo>,
) -> anyhow::Result<()> {
    Ok(execute!(
        stderr(),
        Print("   Compiling ".dark_green().bold()),
        Print(
            package_config
                .name
                .as_ref()
                .map(|name| format!("{name} "))
                .unwrap_or_default()
        ),
        Print(
            package_config
                .version
                .as_ref()
                .map(|version| format!("v{version} "))
                .unwrap_or_default()
        ),
        Print(format!(
            "({})\n",
            git.map(|git| git.url.to_string())
                .unwrap_or_else(|| project_paths.project_path.display().to_string())
        ))
    )?)
}
