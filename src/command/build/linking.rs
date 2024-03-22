use std::{
    env,
    fs::{create_dir_all, read_to_string},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use blake3::Hash;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use hashbrown::{HashMap, HashSet};
use kdam::{tqdm, BarExt, Column, RichProgress, Spinner};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    command::add_mode_path,
    config::{ProjectConfig, SaveConfig},
    file::get_imports,
};

use super::BuildFlags;

pub fn linking(
    project_path: &Path,
    project_config: &ProjectConfig,
    files_to_link: &Vec<(PathBuf, Option<String>, HashSet<PathBuf>)>,
    mut new_hash_hashmap: HashMap<PathBuf, Hash>,
    flags: &BuildFlags,
    stderr: &mut impl Write,
) -> io::Result<()> {
    let mut link_progress_bar_option = if flags.pretty && files_to_link.len() > 0 {
        let mut link_progress_bar = RichProgress::new(
            tqdm!(total = files_to_link.len()),
            vec![
                Column::Text("[bold darkgreen]     Linking".to_string()),
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
        link_progress_bar.refresh().ok();

        Some(link_progress_bar)
    } else {
        None
    };

    let libraries_args = {
        let mut libraries_args = HashMap::new();

        for (library_name, lib_config) in project_config.libraries.iter() {
            let mut args = Vec::new();

            args.push("-L".to_string());

            if let Some(directory) = lib_config.directories.get(0) {
                args.extend([
                    directory.to_string_lossy().to_string(),
                    "-Wl,-rpath".to_string(),
                ]);

                for directory in lib_config.directories.iter() {
                    args.extend([
                        directory.to_string_lossy().to_string(),
                        "-Wl,-rpath".to_string(),
                    ]);
                }
            }

            if cfg!(target_os = "linux") {
                args.extend([
                    "/usr/local/lib/".to_string(),
                    "-Wl,-rpath".to_string(),
                    "/usr/lib/".to_string(),
                    "-Wl,-rpath".to_string(),
                    "/lib/x86_64-linux-gnu/".to_string(),
                    "-Wl,-rpath".to_string(),
                ]);
            }

            args.push(".".to_string());

            for library in lib_config.library.iter() {
                args.push("-l".to_string() + library);
            }

            libraries_args.insert(library_name, args);
        }

        libraries_args
    };

    let commands = files_to_link
        .into_par_iter()
        .map(|(file, lib_name_option, file_to_link)| {
            let mut command = Command::new(&project_config.compiler);

            command
                .current_dir(project_path)
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .arg("-fdiagnostics-color=always");

            if !flags.release {
                command.arg("-g").arg("-Wall");
            } else {
                command.arg("-s");
            }

            if lib_name_option.is_some() {
                command.arg("--shared").arg("-fpic");
            }

            for c_file in file_to_link {
                if let Some(hash) = new_hash_hashmap.get(c_file) {
                    command.arg(
                        add_mode_path(&project_config.objects, flags.release)
                            .join(hash.to_hex().as_str()),
                    );
                } else {
                    return Ok(None);
                }
            }

            let imports = get_imports(&read_to_string(file)?);

            for (library_name, args) in libraries_args.iter() {
                if !imports.contains(library_name) {
                    continue;
                }

                command.args(args);
            }

            let output_path;
            let mut output_file;

            if let Some(lib_name) = lib_name_option {
                output_path = add_mode_path(&project_config.binaries, flags.release);
                output_file = output_path.join(
                    env::consts::FAMILY
                        .eq("unix")
                        .then_some("lib".to_string())
                        .unwrap_or_default()
                        + lib_name,
                );
                output_file.set_extension(env::consts::DLL_EXTENSION);
            } else {
                output_path = add_mode_path(&project_config.binaries, flags.release).join(
                    file.parent()
                        .unwrap_or(Path::new("./"))
                        .strip_prefix(project_path)
                        .unwrap_or(Path::new("./")),
                );
                output_file = output_path.join(file.file_stem().unwrap());
                output_file.set_extension(env::consts::EXE_EXTENSION);
            }

            create_dir_all(project_path.join(output_path))?;

            Ok(Some((
                file,
                command.arg("-o").arg(output_file).spawn().unwrap(),
            )))
        })
        .collect::<Vec<io::Result<Option<(&PathBuf, Child)>>>>();

    let mut errors = Vec::new();

    for command in commands.into_iter() {
        if let Some((file, mut command)) = command? {
            if let Some(link_progress_bar) = &mut link_progress_bar_option {
                link_progress_bar.columns[2] = Column::Text(
                    "[bold blue]".to_string()
                        + &file
                            .strip_prefix(project_path)
                            .unwrap_or(file)
                            .to_string_lossy(),
                );
                link_progress_bar.update(1)?;
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
    }

    new_hash_hashmap.save(project_path)?;

    if let Some(link_progress_bar) = &mut link_progress_bar_option {
        link_progress_bar.columns.drain(1..6);
        link_progress_bar.clear().ok();
        link_progress_bar.refresh().ok();

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
                    Print("\n"),
                    Print(error),
                    Print("\n"),
                )?;
            }
        }
    }

    Ok(())
}
