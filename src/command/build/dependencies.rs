use std::{
    env,
    fs::{copy, create_dir_all, remove_dir_all},
    io::{self, stderr, Read},
    path::Path,
    process::{Command, Stdio},
};

use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use kdam::{tqdm, BarExt, Column, RichProgress, Spinner};

use crate::{
    command::get_project_path,
    config::{LoadConfig, ProjectConfig},
    file::scan_dir_dependency,
};

use super::BuildFlags;

pub fn dependencies(
    project_path: &Path,
    project_config: &ProjectConfig,
    flags: &BuildFlags,
) -> io::Result<()> {
    let mut dependencies_progress_bar_option =
        if flags.pretty && project_config.dependencies.len() > 0 {
            let mut dependencies_progress_bar = RichProgress::new(
                tqdm!(total = project_config.dependencies.len()),
                vec![
                    Column::Text("[bold darkgreen]Dependencies".to_string()),
                    Column::Spinner(Spinner::new(
                        &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
                        80.0,
                        1.0,
                    )),
                    Column::Text("[bold blue]?".to_string()),
                    Column::Animation,
                    Column::Percentage(1),
                    Column::Text("•".to_string()),
                    Column::CountTotal,
                    Column::Text("•".to_string()),
                    Column::ElapsedTime,
                ],
            );
            dependencies_progress_bar.refresh().ok();

            Some(dependencies_progress_bar)
        } else {
            None
        };

    let mut commands = Vec::new();

    for (dependency_name, dependency_path) in project_config.dependencies.iter() {
        let dependency_path = project_path.join(dependency_path);
        let mut command = Command::new(env::current_exe().unwrap());

        command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .arg("build")
            .arg("-f")
            .arg(&dependency_path)
            .arg("--pretty")
            .arg("false");

        if flags.release {
            command.arg("--release");
        }

        if flags.rebuild {
            command.arg("--rebuild");
        }

        commands.push((dependency_name, dependency_path, command.spawn().unwrap()))
    }

    let mut errors = Vec::new();

    for (dependency_name, dependency_path, mut command) in commands.into_iter() {
        if let Some(dependencies_progress_bar) = &mut dependencies_progress_bar_option {
            dependencies_progress_bar.columns[2] =
                Column::Text("[bold blue]".to_string() + &dependency_name);
            dependencies_progress_bar.update(1)?;
        }

        let project_include_path = project_path
            .join(".maky/include/deps")
            .join(dependency_name);

        remove_dir_all(project_path.join(".maky/include"))?;
        create_dir_all(&project_include_path)?;

        if let Ok(exit_code) = command.wait() {
            if exit_code.success() {
                let (dependency_path, dependency_config_path) =
                    &get_project_path(&dependency_path.to_string_lossy());

                if let Ok(mut dependency_config) = ProjectConfig::load(dependency_config_path) {
                    dependency_config.includes.extend(dependency_config.sources);

                    for (_, library) in dependency_config.libraries.into_iter() {
                        dependency_config.includes.extend(library.includes);
                    }

                    for include in dependency_config.includes {
                        let include_path = dependency_path.join(include);

                        if include_path.is_dir() {
                            for h_file in scan_dir_dependency(&include_path)? {
                                copy(
                                    &h_file,
                                    project_include_path
                                        .join(h_file.strip_prefix(&include_path).unwrap()),
                                )?;
                            }
                        }
                    }
                }
            }

            let mut buffer = String::new();

            if let Some(stderr) = command.stderr.as_mut() {
                stderr.read_to_string(&mut buffer)?;
                errors.push((dependency_name.clone(), buffer));
            }
        }
    }

    if let Some(dependencies_progress_bar) = &mut dependencies_progress_bar_option {
        dependencies_progress_bar.columns.drain(1..6);
        dependencies_progress_bar.clear().ok();
        dependencies_progress_bar.refresh().ok();

        println!();
    }

    if !errors.is_empty() {
        let mut is_first = true;

        for (dependency_name, error) in errors.iter() {
            let error = error.trim_end();

            if !error.is_empty() {
                if is_first {
                    eprintln!();

                    is_first = false;
                }

                execute!(
                    stderr(),
                    SetForegroundColor(Color::Red),
                    Print("Errors : ".bold()),
                    ResetColor,
                    SetForegroundColor(Color::Cyan),
                    Print(dependency_name.clone().bold()),
                    ResetColor,
                )?;

                eprintln!("{error}\n");
            }
        }
    }

    Ok(())
}
