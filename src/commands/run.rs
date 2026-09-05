use std::{
    io::stderr,
    process::{ExitCode, Stdio},
};

use anyhow::anyhow;
use crossterm::{
    execute,
    style::{Print, Stylize},
};
use tokio::process;

use crate::{
    commands::{self, ARGUMENTS_SELECTION, COMPILATION_OPTIONS, TARGET_SELECTION, build},
    config::{Target, TargetType},
    helpers::PathTarget,
};

#[derive(clap::Args, Debug, Clone)]
pub struct Command {
    /// Arguments for the binary or example to run
    #[arg(help_heading = ARGUMENTS_SELECTION)]
    args: Vec<String>,

    /// Name of the bin target to run
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    bin: Option<String>,

    /// Name of the example target to run
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    example: Option<String>,

    /// Build artifacts in release mode, with optimizations
    #[arg(short, long, help_heading = COMPILATION_OPTIONS)]
    release: bool,

    /// Update dependencies
    #[arg(short, long, help_heading = COMPILATION_OPTIONS)]
    update: bool,

    #[command(flatten)]
    project: commands::ProjectArgs,
}

impl Command {
    pub async fn execute(self) -> anyhow::Result<ExitCode> {
        let build_command = build::Command {
            lib: false,
            bins: false,
            bin: Vec::from_iter(self.bin.clone()),
            examples: false,
            example: Vec::from_iter(self.example.clone()),
            tests: false,
            test: Vec::new(),
            benches: false,
            bench: Vec::new(),
            all_targets: false,
            release: self.release,
            update: self.update,
            project: self.project,
        };
        let (_, project_paths, package_config, targets, _) = build_command.build().await?;
        let mut binary_targets: Vec<Target> = targets
            .into_iter()
            .filter(|target| {
                target.target_type == TargetType::Bin
                    && ((self.bin.is_none() && self.example.is_none())
                        || target.name().ok() == self.bin.clone().or(self.example.clone()))
            })
            .collect();

        if binary_targets.len() > 1 {
            return Err(anyhow!(
                "`maky run` could not determine which binary to run"
            ));
        }

        let Some(binary_target) = binary_targets.pop() else {
            return Err(anyhow!("a bin target must be available for `maky run`"));
        };

        let command_path = package_config
            .binaries()
            .target_release(self.release)
            .join(binary_target.binary_name()?);

        if !project_paths.project_path.join(&command_path).is_file() {
            return Ok(ExitCode::SUCCESS);
        }

        let mut command = process::Command::new(&command_path);

        command
            .kill_on_drop(true)
            .current_dir(&project_paths.project_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit())
            .args(&self.args);

        execute!(
            stderr(),
            Print("     Running ".dark_green().bold()),
            Print("`"),
            Print(command_path.display()),
            Print(if self.args.is_empty() { "" } else { " " }),
            Print(self.args.join(" ")),
            Print("`"),
            Print("\n")
        )?;

        if let Some(code) = command.status().await?.code()
            && code != 0
        {
            Ok(ExitCode::from(code as u8))
        } else {
            Ok(ExitCode::SUCCESS)
        }
    }
}
