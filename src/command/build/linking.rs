use std::{
    env,
    fs::create_dir_all,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use aho_corasick::AhoCorasick;
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
};

use super::BuildFlags;

pub fn linking(
    project_path: &Path,
    project_config: &ProjectConfig,
    import_hashmap: &HashMap<PathBuf, Vec<String>>,
    files_to_link: &Vec<(PathBuf, bool, Option<String>, HashSet<PathBuf>)>,
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
        .map(|(file, is_library, name_option, file_to_link)| {
            let mut command = Command::new(project_config.get_compiler(file).unwrap());

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

            if *is_library {
                command.arg("--shared");
            }

            let mut o_c_link = Vec::new();

            for c_file in file_to_link {
                if let Some(hash) = new_hash_hashmap.get(c_file) {
                    let o_file = add_mode_path(&project_config.objects, flags.release)
                        .join(hash.to_hex().as_str());

                    command.arg(&o_file);
                    o_c_link.push((
                        o_file.to_string_lossy().to_string(),
                        c_file.to_string_lossy().to_string(),
                    ));
                } else {
                    return Ok(None);
                }
            }

            if let Some(imports) = import_hashmap.get(file) {
                for (library_name, args) in libraries_args.iter() {
                    if !imports.contains(library_name) {
                        continue;
                    }

                    command.args(args);
                }
            }

            let output_path = add_mode_path(&project_config.binaries, flags.release);
            let mut output_file;
            let name = name_option
                .clone()
                .unwrap_or(file.file_stem().unwrap().to_string_lossy().to_string());

            if *is_library {
                output_file = output_path.join(env::consts::DLL_PREFIX.to_string() + &name);
                output_file.set_extension(env::consts::DLL_EXTENSION);
            } else {
                output_file = output_path.join(name);
                output_file.set_extension(env::consts::EXE_EXTENSION);
            }

            create_dir_all(project_path.join(output_path))?;

            Ok(Some((
                file,
                command.arg("-o").arg(output_file).spawn().unwrap(),
                o_c_link,
            )))
        })
        .collect::<Vec<io::Result<Option<(&PathBuf, Child, Vec<(String, String)>)>>>>();

    let mut errors = Vec::new();

    for command in commands.into_iter() {
        if let Some((file, mut command, o_c_link)) = command? {
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
                    errors.push((file, buffer, o_c_link));
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

        for (file, error, o_c_link) in errors.into_iter() {
            if !error.is_empty() {
                if is_first {
                    writeln!(stderr)?;

                    is_first = false;
                }

                let (o_file, c_file): (Vec<String>, Vec<String>) = o_c_link.into_iter().unzip();
                let mut formatted_error = Vec::new();

                AhoCorasick::new(&o_file)
                    .expect("Failed to initialize AhoCorasick pattern matcher")
                    .try_stream_replace_all(error.as_bytes(), &mut formatted_error, &c_file)?;

                execute!(
                    stderr,
                    SetForegroundColor(Color::Red),
                    Print("Errors : ".bold()),
                    ResetColor,
                    Print(file.to_string_lossy()),
                    Print("\n\n"),
                    Print(String::from_utf8_lossy(&formatted_error)),
                    Print("\n"),
                )?;
            }
        }
    }

    Ok(())
}
