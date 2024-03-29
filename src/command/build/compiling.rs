use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use blake3::Hash;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use hashbrown::HashMap;
use kdam::{tqdm, BarExt, Column, RichProgress, Spinner};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{command::add_mode_path, config::ProjectConfig};

use super::BuildFlags;

pub fn compiling(
    project_path: &Path,
    project_config: &ProjectConfig,
    files_to_compile: &HashMap<PathBuf, Hash>,
    new_hash_hashmap: &mut HashMap<PathBuf, Hash>,
    flags: &BuildFlags,
    stderr: &mut impl Write,
) -> io::Result<()> {
    let mut compile_progress_bar_option = if flags.pretty && files_to_compile.len() > 0 {
        let mut compile_progress_bar = RichProgress::new(
            tqdm!(total = files_to_compile.len()),
            vec![
                Column::Text("[bold darkgreen]   Compiling".to_string()),
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
        compile_progress_bar.refresh().ok();

        Some(compile_progress_bar)
    } else {
        None
    };

    let include_args = {
        let mut include_args = Vec::new();

        for include in project_config.includes.iter() {
            include_args.push("-I".to_string());
            include_args.push(include.to_string_lossy().to_string());
        }

        include_args
    };

    let commands = files_to_compile
        .into_par_iter()
        .map(|(file_path, file_hash)| {
            let mut command = Command::new(&project_config.compiler);

            command
                .current_dir(project_path)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .arg("-fdiagnostics-color=always");

            if let Some(standard) = project_config.standard.as_ref() {
                command.arg(format!("-std={standard}"));
            }

            if !flags.release {
                command.arg("-O0").arg("-g").arg("-Wall");
            } else {
                command.arg("-O2");
            }

            (
                file_path,
                command
                    .args(&include_args)
                    .arg("-c")
                    .arg(file_path.strip_prefix(project_path).unwrap())
                    .arg("-o")
                    .arg(
                        add_mode_path(&project_config.objects, flags.release)
                            .join(file_hash.to_hex().as_str()),
                    )
                    .spawn()
                    .unwrap(),
            )
        })
        .collect::<HashMap<&PathBuf, Child>>();

    let mut errors = Vec::new();

    for (file, mut command) in commands.into_iter() {
        if let Some(compile_progress_bar) = &mut compile_progress_bar_option {
            compile_progress_bar.columns[2] = Column::Text(
                "[bold blue]".to_string()
                    + &file
                        .strip_prefix(project_path)
                        .unwrap_or(&file)
                        .to_string_lossy(),
            );
            compile_progress_bar.update(1)?;
        }

        if let Ok(exit_code) = command.wait() {
            if !exit_code.success() {
                new_hash_hashmap.remove(file);
            }

            let mut buffer = String::new();

            if let Some(stderr) = command.stderr.as_mut() {
                stderr.read_to_string(&mut buffer)?;
                errors.push((file, buffer));
            }
        } else {
            new_hash_hashmap.remove(file);
        }
    }

    if let Some(compile_progress_bar) = &mut compile_progress_bar_option {
        compile_progress_bar.columns.drain(1..6);
        compile_progress_bar.clear().ok();
        compile_progress_bar.refresh().ok();

        println!();
    }

    if !errors.is_empty() {
        let mut is_first = true;

        for (file, error) in errors.iter() {
            if !error.is_empty() {
                if is_first {
                    writeln!(stderr)?;

                    is_first = false;
                }

                execute!(
                    stderr,
                    SetForegroundColor(Color::Red),
                    Print("Errors : ".bold()),
                    ResetColor,
                    Print(file.to_string_lossy()),
                    Print("\n\n"),
                    Print(error),
                    Print("\n")
                )?;
            }
        }
    }

    Ok(())
}
