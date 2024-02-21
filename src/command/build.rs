use std::{
    env,
    fs::{create_dir, create_dir_all, read_dir, remove_dir, remove_file, File},
    io::{self, stderr, stdout, Read},
    path::Path,
    process::{Command, Stdio},
    time::Instant,
};

use ahash::{AHashMap, AHashSet};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use kdam::{tqdm, BarExt, Column, RichProgress, Spinner};

use crate::{
    config::{Config, LoadConfig, ProjectConfig, SaveConfig},
    file::{
        compile::compile,
        link::{get_imports, link},
        scan_dir,
    },
};

pub fn build(config_file: String, release: bool) -> io::Result<()> {
    let project_config_path = Path::new(&config_file);
    let project_path = project_config_path.parent().unwrap_or(Path::new("./"));
    let time = Instant::now();

    execute!(
        stdout(),
        SetForegroundColor(Color::parse_ansi("2;118;200;56").unwrap()),
        Print(r"              _          ".to_string() + "\n"),
        Print(r"  /\/\   __ _| | ___   _ ".to_string() + "\n"),
        SetForegroundColor(Color::parse_ansi("2;101;180;48").unwrap()),
        Print(r" /    \ / _` | |/ / | | |".to_string() + "\n"),
        SetForegroundColor(Color::parse_ansi("2;78;150;37").unwrap()),
        Print(r"/ /\/\ \ (_| |   <| |_| |".to_string() + "\n"),
        SetForegroundColor(Color::parse_ansi("2;54;120;26").unwrap()),
        Print(r"\/    \/\__,_|_|\_\\__, |".to_string() + "\n"),
        SetForegroundColor(Color::DarkMagenta),
        if release {
            Print(r"Release             ".bold())
        } else {
            Print(r"Dev                 ".bold())
        },
        SetForegroundColor(Color::parse_ansi("2;24;80;11").unwrap()),
        Print(r"|___/".to_string() + "\n\n"),
        ResetColor
    )?;

    match ProjectConfig::load(project_config_path) {
        Ok(mut project_config) => {
            let dir_path = project_path.join("./.maky");
            if !dir_path.is_dir() {
                create_dir(dir_path)?;
            }

            let binaries_dir_path = project_path.join(&project_config.binaries);
            if !binaries_dir_path.is_dir() {
                create_dir(&binaries_dir_path)?;
            }

            for source in project_config.sources.iter() {
                let sources_dir_path = project_path.join(source);
                if !sources_dir_path.is_dir() {
                    create_dir(sources_dir_path)?;
                }

                project_config.includes.push(source.clone());
            }

            let objects_dir_path = project_path.join(&project_config.objects);
            if !objects_dir_path.is_dir() {
                create_dir(&objects_dir_path)?;
            }

            for library in project_config.libraries.values() {
                project_config.includes.extend_from_slice(&library.includes);
            }

            for include in project_config.includes.iter_mut() {
                *include = project_path.join(&include);
            }

            if cfg!(target_os = "linux") {
                project_config.includes.push("/usr/include".into());
            }

            project_config.includes.dedup();

            let mut config = Config::load(project_path).unwrap_or_default();
            let mut hash_hashmap = if config.release != release {
                for entry in read_dir(&objects_dir_path)? {
                    if let Ok(entry) = entry {
                        let path = entry.path();

                        if path.is_file() {
                            remove_file(path)?;
                        } else if path.is_dir() {
                            remove_dir(path)?;
                        }
                    }
                }

                AHashMap::new()
            } else {
                AHashMap::load(project_path).unwrap_or_default()
            };
            let mut new_hash_hashmap = AHashMap::new();
            let mut main_hashset = AHashSet::new();
            let mut h_h_link = AHashMap::new();
            let mut h_c_link = AHashMap::new();
            let mut c_h_link = AHashMap::new();

            config.release = release;
            config.save(project_path)?;

            for source in project_config.sources.iter() {
                scan_dir(
                    &project_config,
                    &project_path.join(source),
                    &mut main_hashset,
                    &mut h_h_link,
                    &mut h_c_link,
                    &mut c_h_link,
                    &mut new_hash_hashmap,
                )?;
            }

            let files_to_compile = compile(
                &objects_dir_path,
                &h_h_link,
                &h_c_link,
                &mut hash_hashmap,
                &new_hash_hashmap,
            );

            let mut compile_progress_bar_option = if files_to_compile.len() > 0 {
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

            let mut commands = vec![];
            let mut include_args = vec![];

            for include in project_config.includes.iter() {
                include_args.push("-I".to_string());
                include_args.push(
                    include
                        .strip_prefix(project_path)
                        .unwrap_or(include)
                        .to_string_lossy()
                        .to_string(),
                );
            }

            for file in files_to_compile.iter() {
                let mut command = Command::new(&project_config.compiler);

                command
                    .current_dir(project_path)
                    .stderr(Stdio::piped())
                    .arg("-fdiagnostics-color=always");

                if !release {
                    command.arg("-O0").arg("-g").arg("-Wall");
                } else {
                    command.arg("-O2");
                }

                commands.push((
                    file.0,
                    command
                        .args(&include_args)
                        .arg("-c")
                        .arg(file.0.strip_prefix(project_path).unwrap())
                        .arg("-o")
                        .arg(project_config.objects.join(file.1.to_hex().as_str()))
                        .spawn()
                        .unwrap(),
                ));
            }

            let files_to_link = link(
                &project_config,
                &main_hashset,
                &files_to_compile,
                &h_c_link,
                &c_h_link,
            )?;

            let mut errors = vec![];

            for (file, mut command) in commands.drain(..) {
                if let Some(compile_progress_bar) = &mut compile_progress_bar_option {
                    compile_progress_bar.columns[2] = Column::Text(
                        "[bold blue]".to_string()
                            + &file
                                .strip_prefix(project_path)
                                .unwrap_or(&file)
                                .to_string_lossy(),
                    );
                    compile_progress_bar.update(1).ok();
                }

                if let Ok(exit_code) = command.wait() {
                    if !exit_code.success() {
                        new_hash_hashmap.remove(file);
                    }

                    let mut buffer = String::new();

                    command.stderr.unwrap().read_to_string(&mut buffer)?;
                    errors.push((file, buffer));
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
                            println!();

                            is_first = false;
                        }

                        execute!(
                            stderr(),
                            SetForegroundColor(Color::Red),
                            Print("Errors : ".bold()),
                            ResetColor,
                            Print(file.to_string_lossy()),
                            Print("\n\n"),
                        )?;

                        eprintln!("{error}");
                    }
                }
            }

            let mut link_progress_bar_option = if files_to_link.len() > 0 {
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

            for (main_file, file_to_link) in &files_to_link {
                let mut command = Command::new(&project_config.compiler);

                command
                    .current_dir(project_path)
                    .stderr(Stdio::piped())
                    .arg("-fdiagnostics-color=always");

                if !release {
                    command.arg("-g").arg("-Wall");
                } else {
                    command.arg("-s");
                }

                for c_file in file_to_link {
                    if let Some(hash) = new_hash_hashmap.get(c_file) {
                        command.arg(project_config.objects.join(hash.to_hex().as_str()));
                    } else {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Object file for `{}` not found.", &c_file.to_string_lossy()),
                        ));
                    }
                }

                let imports = get_imports(&{
                    let mut code = String::new();

                    File::open(main_file)?.read_to_string(&mut code)?;

                    code
                });

                for (library_name, lib_config) in project_config.libraries.iter_mut() {
                    if !imports.contains(library_name) {
                        continue;
                    }

                    command.arg("-L");

                    if let Some(directory) = lib_config.directories.get(0) {
                        command.arg(directory).arg("-Wl,-rpath");
                    }

                    for directory in lib_config.directories.iter() {
                        command.arg(directory).arg("-Wl,-rpath");
                    }

                    if cfg!(target_os = "linux") {
                        command
                            .arg("/usr/local/lib/")
                            .arg("-Wl,-rpath")
                            .arg("/usr/lib/")
                            .arg("-Wl,-rpath")
                            .arg("/lib/x86_64-linux-gnu/")
                            .arg("-Wl,-rpath");
                    }

                    command.arg(".");

                    for library in lib_config.library.iter() {
                        command.arg("-l".to_string() + library);
                    }
                }

                let output_path = project_config
                    .binaries
                    .join(if release {
                        Path::new("release")
                    } else {
                        Path::new("debug")
                    })
                    .join(
                        main_file
                            .parent()
                            .unwrap_or(Path::new("./"))
                            .strip_prefix(project_path)
                            .unwrap_or(Path::new("./")),
                    );
                let mut output_file = output_path.join(main_file.file_stem().unwrap());

                create_dir_all(project_path.join(output_path)).unwrap();

                output_file.set_extension(env::consts::EXE_EXTENSION);

                commands.push((
                    main_file,
                    command.arg("-o").arg(output_file).spawn().unwrap(),
                ));
            }

            let mut errors = vec![];

            for (file, mut command) in commands.drain(..) {
                if let Some(link_progress_bar) = &mut link_progress_bar_option {
                    link_progress_bar.columns[2] = Column::Text(
                        "[bold blue]".to_string()
                            + &file
                                .strip_prefix(project_path)
                                .unwrap_or(file)
                                .to_string_lossy(),
                    );
                    link_progress_bar.update(1).ok();
                }

                if let Ok(exit_code) = command.wait() {
                    if !exit_code.success() {
                        new_hash_hashmap.remove(file);
                    }

                    let mut buffer = String::new();

                    command.stderr.unwrap().read_to_string(&mut buffer)?;
                    errors.push((file, buffer));
                } else {
                    new_hash_hashmap.remove(file);
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
                            println!();

                            is_first = false;
                        }

                        execute!(
                            stderr(),
                            SetForegroundColor(Color::Red),
                            Print("Errors : ".bold()),
                            ResetColor,
                            Print(file.to_string_lossy()),
                            Print("\n"),
                        )?;

                        eprintln!("{error}");
                    }
                }
            }

            execute!(
                stdout(),
                SetForegroundColor(Color::DarkGreen),
                Print("    Finished ".bold()),
                ResetColor,
                Print(if release {
                    "release [optimized]"
                } else {
                    "dev [unoptimized + debuginfo]"
                }),
                Print(format!(" target(s) in {:.2?}\n", time.elapsed()))
            )?;
        }
        Err(error) => ProjectConfig::handle_error(error, project_config_path)?,
    }

    Ok(())
}
