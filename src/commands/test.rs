use std::{
    io::{Write, stderr, stdout},
    process::{ExitCode, Stdio},
    time::Duration,
};

use crossterm::{
    execute,
    style::{Print, Stylize},
};
use tokio::{io::AsyncReadExt, process, select, spawn, task::JoinHandle, time::Instant};

use crate::{
    commands::{self, ARGUMENTS_SELECTION, COMPILATION_OPTIONS, TARGET_SELECTION, build},
    config::{Target, TargetType},
    helpers::PathTarget,
};

#[derive(clap::Args, Debug, Clone)]
pub struct Command {
    /// If specified, only run tests containing this string in their names
    #[arg(value_name = "TESTNAME", help_heading = ARGUMENTS_SELECTION)]
    testname: Option<String>,

    /// Arguments for the binary or example to run
    #[arg(help_heading = ARGUMENTS_SELECTION)]
    args: Vec<String>,

    /// Compile, but don't run tests
    #[arg(long)]
    no_run: bool,

    /// Build only the specified test target
    #[arg(long, value_name = "NAME", help_heading = TARGET_SELECTION)]
    test: Vec<String>,

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
            bin: Vec::new(),
            examples: false,
            example: Vec::new(),
            tests: self.test.is_empty(),
            test: self.test.clone(),
            benches: false,
            bench: Vec::new(),
            all_targets: false,
            release: self.release,
            update: self.update,
            project: self.project,
        };
        let (_, project_paths, package_config, targets, _) = build_command.build().await?;

        let unfiltered_test_target_count = targets.len();
        let test_targets: Vec<Target> = targets
            .into_iter()
            .filter(|target| {
                target.target_type == TargetType::Bin
                    && (self.testname.is_none()
                        || target
                            .name()
                            .ok()
                            .zip(self.testname.as_ref())
                            .map_or(false, |(name, test)| name.contains(test)))
            })
            .collect();

        if self.no_run {
            return Ok(ExitCode::SUCCESS);
        }

        execute!(
            stderr(),
            Print("     Running ".dark_green().bold()),
            Print(" unittests"),
            Print("\n"),
        )?;
        println!("\nrunning {} tests", test_targets.len());

        let mut passed = Vec::new();
        let mut failed = Vec::new();
        let mut duration = Duration::default();

        for test_target in &test_targets {
            let name = test_target.name().unwrap_or(test_target.path.clone());

            print!("test {name} ...");
            stdout().flush()?;

            let command_path = package_config
                .binaries()
                .target_release(self.release)
                .join(test_target.binary_name()?);

            if !project_paths.project_path.join(&command_path).is_file() {
                failed.push((name, Vec::new()));
                execute!(stdout(), Print(" FAILED".red()), Print("\n"))?;
                continue;
            }

            let mut command = process::Command::new(&command_path);

            command
                .kill_on_drop(true)
                .current_dir(&project_paths.project_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .args(&self.args);

            let time = Instant::now();
            let mut child = command.spawn()?;
            let mut child_stdout = child.stdout.take().unwrap();
            let mut child_stderr = child.stderr.take().unwrap();

            let child_std_buffer: JoinHandle<Vec<u8>> = spawn(async move {
                let mut child_std_buffer = Vec::new();

                loop {
                    let mut child_stdout_buffer = [0; 2048];
                    let mut child_stderr_buffer = [0; 2048];

                    let (length, new_child_std_buffer) = select! {
                        length = child_stdout.read(&mut child_stdout_buffer) => {
                            (length, child_stdout_buffer)
                        },
                        length = child_stderr.read(&mut child_stderr_buffer) => {
                            (length, child_stderr_buffer)
                        },
                    };

                    if let Ok(length) = length
                        && length > 0
                    {
                        child_std_buffer.extend(&new_child_std_buffer[..length]);
                    } else {
                        return child_std_buffer;
                    }
                }
            });

            let exit_status = child.wait().await?;

            duration += time.elapsed();

            if exit_status.success() {
                passed.push((name, child_std_buffer.await?));
                execute!(stdout(), Print(" ok".dark_green()), Print("\n"))?;
            } else {
                failed.push((name, child_std_buffer.await?));
                execute!(stdout(), Print(" FAILED".red()), Print("\n"))?;
            }
        }

        if failed.len() > 0 {
            println!("\nfailures:");

            for (name, child_std_buffer) in &failed {
                println!("\n---- {name} ----");
                stdout().write_all(child_std_buffer)?;
                stdout().flush()?;
            }

            println!("\nfailures:");

            for (name, _) in &failed {
                println!("    {name}");
            }
        }

        execute!(
            stdout(),
            Print("\n"),
            Print("test result: "),
            Print(if failed.len() > 0 {
                "FAILED".red()
            } else {
                "ok".dark_green()
            }),
            Print(format!(
                ". {} passed; {} failed; {} filtered out; finished in {:.2?}",
                passed.len(),
                failed.len(),
                unfiltered_test_target_count - test_targets.len(),
                duration,
            )),
            Print("\n\n"),
        )?;

        if failed.len() > 0 {
            return Ok(ExitCode::FAILURE);
        }

        Ok(ExitCode::SUCCESS)
    }
}
