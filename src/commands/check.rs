use std::{collections::HashSet, io::stderr, path::PathBuf, time::Instant};

use crossterm::{
    execute,
    style::{Print, Stylize},
};

use crate::{
    commands::{
        self, COMPILATION_OPTIONS, TARGET_SELECTION,
        build::{
            Dependency, compile, get_source_files_reverse_dependencies, get_targets_source_files,
            scan_source_files,
        },
    },
    helpers,
};

#[derive(clap::Args, Debug, Clone)]
pub struct Command {
    /// Check only this package's library
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub lib: bool,

    /// Check all binaries
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub bins: bool,

    /// Check only the specified binary
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub bin: Vec<String>,

    /// Check all examples
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub examples: bool,

    /// Check only the specified example
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub example: Vec<String>,

    /// Check all tests
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub tests: bool,

    /// Check only the specified test target
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub test: Vec<String>,

    /// Check all benches
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub benches: bool,

    /// Check only the specified bench target
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    pub bench: Vec<String>,

    /// Check all targets
    #[arg(long, help_heading = TARGET_SELECTION)]
    pub all_targets: bool,

    /// Check artifacts in release mode, with optimizations
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

        let include_paths = project_paths.include_paths(&package_config);

        let source_files =
            scan_source_files(&targets, &project_paths, &package_config, &include_paths).await?;
        let source_files_reverse_dependencies =
            get_source_files_reverse_dependencies(&source_files);

        let targets_source_files =
            get_targets_source_files(&targets, &source_files, &source_files_reverse_dependencies)
                .await?;
        let targets_source_files: HashSet<PathBuf> =
            targets_source_files.into_values().flatten().collect();

        compile(
            &project_paths,
            &package_config,
            &dependencies,
            &targets_source_files,
            &source_files,
            self.release,
            &vec![
                "-fsyntax-only".to_string(),
                "-Wall".to_string(),
                "-Wextra".to_string(),
                "-Wno-unused-command-line-argument".to_string(),
            ],
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

        Ok(())
    }
}
