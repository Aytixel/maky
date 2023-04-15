mod compile;
mod config;
mod link;

use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir, create_dir_all, read_dir, read_to_string, remove_dir, remove_file},
    io::{self, stderr, stdout, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

use blake3::{hash, Hash};
use clap::{command, Parser, Subcommand};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use kdam::{tqdm, BarExt, Column, RichProgress};
use regex::Regex;

use crate::{
    compile::compile,
    config::{Config, LibConfig, LoadConfig, ProjectConfig, SaveConfig},
    link::link,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Maky config file
    #[arg(short, long, default_value_t = ("./Maky.toml").to_string())]
    file: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build files
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },

    /// Build files then run the specified file
    Run {
        /// Build in release mode
        #[arg(long)]
        release: bool,

        /// Path of the source file to build and run
        file: PathBuf,

        /// Arguments for the source file to run
        args: Vec<String>,
    },
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let project_config_path = Path::new(&args.file);
    let project_path = project_config_path.parent().unwrap_or(Path::new("./"));

    if let Some(command) = args.command {
        let release = match command {
            Commands::Build { release } | Commands::Run { release, .. } => release,
        };
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

        if let Ok(mut project_config) = ProjectConfig::load(project_config_path) {
            let dir_path = project_path.join("./.maky");
            if !dir_path.is_dir() {
                create_dir(dir_path)?;
            }

            let binaries_dir_path = project_path.join(project_config.binaries.clone());
            if !binaries_dir_path.is_dir() {
                create_dir(binaries_dir_path.clone())?;
            }

            for source in project_config.sources.iter() {
                let sources_dir_path = project_path.join(source);

                if !sources_dir_path.is_dir() {
                    create_dir(sources_dir_path)?;
                }

                project_config.includes.push(project_path.join(source));
            }

            let objects_dir_path = project_path.join(project_config.objects.clone());
            if !objects_dir_path.is_dir() {
                create_dir(objects_dir_path.clone())?;
            }

            project_config
                .includes
                .push(project_path.join("/usr/include"));

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

                HashMap::new()
            } else {
                HashMap::load(project_path).unwrap_or_default()
            };
            let mut new_hash_hashmap = HashMap::new();
            let mut main_hashset = HashSet::new();
            let mut h_h_link = HashMap::new();
            let mut h_c_link = HashMap::new();
            let mut c_h_link = HashMap::new();

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
                        Column::text("[bold darkgreen]   Compiling"),
                        Column::Spinner(
                            "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
                                .chars()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>(),
                            80.0,
                            1.0,
                        ),
                        Column::text("[bold blue]?"),
                        Column::Bar,
                        Column::Percentage(1),
                        Column::text("•"),
                        Column::CountTotal,
                        Column::text("•"),
                        Column::ElapsedTime,
                    ],
                );
                compile_progress_bar.refresh();

                Some(compile_progress_bar)
            } else {
                None
            };

            let mut commands = vec![];

            for file in files_to_compile.iter() {
                let mut command = Command::new(&project_config.compiler);

                command
                    .stderr(Stdio::piped())
                    .arg("-fdiagnostics-color=always");

                if !release {
                    command.arg("-g").arg("-Wall");
                }

                for include in project_config.includes.iter() {
                    command.arg("-I").arg(include);
                }

                commands.push((
                    file.0,
                    command
                        .arg("-c")
                        .arg(file.0)
                        .arg("-o")
                        .arg(objects_dir_path.join(file.1.to_hex().as_str()))
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
            );

            let mut errors = vec![];

            for (file, mut command) in commands.drain(..) {
                if let Some(compile_progress_bar) = &mut compile_progress_bar_option {
                    compile_progress_bar.columns[2] = Column::text(
                        &("[bold blue]".to_string()
                            + &file
                                .strip_prefix(project_path)
                                .unwrap_or(file)
                                .to_string_lossy()),
                    );
                    compile_progress_bar.update(1);
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
                compile_progress_bar.clear();
                compile_progress_bar.refresh();

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
                        Column::text("[bold darkgreen]     Linking"),
                        Column::Spinner(
                            "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"
                                .chars()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>(),
                            80.0,
                            1.0,
                        ),
                        Column::text("[bold blue]?"),
                        Column::Bar,
                        Column::Percentage(1),
                        Column::text("•"),
                        Column::CountTotal,
                        Column::text("•"),
                        Column::ElapsedTime,
                    ],
                );
                link_progress_bar.refresh();

                Some(link_progress_bar)
            } else {
                None
            };

            for (main_file, file_to_link) in &files_to_link {
                let mut command = Command::new(&project_config.compiler);

                command
                    .stderr(Stdio::piped())
                    .arg("-fdiagnostics-color=always");

                if !release {
                    command.arg("-g").arg("-Wall");
                }

                for c_file in file_to_link {
                    command.arg(objects_dir_path.join(new_hash_hashmap[c_file].to_hex().as_str()));
                }

                command
                    .arg("-L")
                    .arg("/usr/local/lib/")
                    .arg("-Wl,-rpath")
                    .arg("/usr/lib/")
                    .arg("-Wl,-rpath")
                    .arg("/lib/x86_64-linux-gnu/");

                for lib in {
                    let mut libs: Vec<&LibConfig> = project_config.libraries.values().collect();

                    libs.reverse();
                    libs
                } {
                    if !lib.regex.is_empty() {
                        if !lib
                            .regex
                            .iter()
                            .any(|regex| regex.is_match(&main_file.to_string_lossy()))
                        {
                            continue;
                        }
                    }

                    let mut lib_dir_iter = lib.directories.iter();

                    if let Some(lib_dir) = lib_dir_iter.next() {
                        command.arg("-L").arg(lib_dir);

                        for lib_dir in lib_dir_iter {
                            command.arg("-Wl,-rpath").arg(lib_dir);
                        }
                    }

                    for lib in lib.library.iter() {
                        command.arg("-l".to_string() + lib);
                    }
                }

                let output_path = binaries_dir_path
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

                create_dir_all(output_path).unwrap();

                if cfg!(windows) {
                    output_file.set_extension("exe");
                }

                commands.push((
                    main_file,
                    command.arg("-o").arg(output_file).spawn().unwrap(),
                ));
            }

            let mut errors = vec![];

            for (file, mut command) in commands.drain(..) {
                if let Some(link_progress_bar) = &mut link_progress_bar_option {
                    link_progress_bar.columns[2] = Column::text(
                        &("[bold blue]".to_string()
                            + &file
                                .strip_prefix(project_path)
                                .unwrap_or(file)
                                .to_string_lossy()),
                    );
                    link_progress_bar.update(1);
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
                link_progress_bar.clear();
                link_progress_bar.refresh();

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

            if let Commands::Run { file, args, .. } = command {
                let mut output_file = binaries_dir_path
                    .join(if release {
                        Path::new("release")
                    } else {
                        Path::new("debug")
                    })
                    .join(file);

                if cfg!(windows) {
                    output_file.set_extension("exe");
                }

                if output_file.exists() {
                    execute!(
                        stdout(),
                        SetForegroundColor(Color::DarkGreen),
                        Print("     Running ".bold()),
                        ResetColor,
                        Print("`"),
                        Print(output_file.to_string_lossy()),
                        Print(
                            match args
                                .clone()
                                .drain(..)
                                .reduce(|accumulator, arg| accumulator + " " + &arg)
                            {
                                Some(value) => " ".to_string() + &value,
                                None => "".to_string(),
                            }
                        ),
                        Print("`\n")
                    )?;

                    Command::new(output_file)
                        .args(args)
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .output()
                        .unwrap();
                }
            }

            return Ok(());
        }

        execute!(
            stderr(),
            SetForegroundColor(Color::DarkRed),
            Print("Project config file found !\n".bold()),
            ResetColor,
        )?;
    }

    return Ok(());
}

fn scan_dir(
    project_config: &ProjectConfig,
    dir_path: &Path,
    main_hashset: &mut HashSet<PathBuf>,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    for entry in read_dir(dir_path)? {
        if let Ok(entry) = entry {
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().unwrap_or_default();

                if extension == "c" || extension == "h" {
                    let code = read_to_string(&path).unwrap();
                    let includes = get_includes(&path, project_config.includes.clone(), &code);

                    if extension == "c" {
                        c_h_link.insert(path.to_path_buf(), includes.clone());

                        if has_main(&code) {
                            main_hashset.insert(path.to_path_buf());
                        }
                    }

                    hash_hashmap.insert(path.to_path_buf(), hash(code.as_bytes()));

                    for include in includes {
                        if extension == "c" {
                            h_c_link
                                .entry(include)
                                .or_insert(HashSet::new())
                                .insert(path.clone());
                        } else {
                            h_h_link
                                .entry(include)
                                .or_insert(HashSet::new())
                                .insert(path.clone());
                        }
                    }
                }
            } else if path.is_dir() {
                scan_dir(
                    project_config,
                    &path,
                    main_hashset,
                    h_h_link,
                    h_c_link,
                    c_h_link,
                    hash_hashmap,
                )?;
            }
        }
    }

    return Ok(());
}

fn get_includes(path: &Path, include_path_vec: Vec<PathBuf>, code: &String) -> HashSet<PathBuf> {
    let mut include_hashset = HashSet::new();
    let parent_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();

    for line in code.lines() {
        let line = line.trim();

        if line.starts_with("#include") {
            for include in Regex::new("(\"(.*)\"|<(.*)>)").unwrap().captures_iter(line) {
                if let Some(include) = include[0].get(1..include[0].len() - 1) {
                    let path = Path::new(include);

                    if path.is_file() {
                        include_hashset.insert(path.to_path_buf());
                        continue;
                    }

                    let path_with_parent = parent_path.join(path);

                    if path_with_parent.is_file() {
                        include_hashset.insert(path_with_parent.to_path_buf());
                        continue;
                    }

                    for include_path in include_path_vec.clone() {
                        let path = include_path.join(path);

                        if path.is_file() {
                            include_hashset.insert(path.to_path_buf());
                            continue;
                        }
                    }
                }
            }
        }
    }

    include_hashset
}

fn has_main(code: &String) -> bool {
    Regex::new(r"(void|int)[ \t\n\r]*main[ \t\n\r]*\(")
        .unwrap()
        .is_match(&code)
}
