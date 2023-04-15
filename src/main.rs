mod compile;
mod config;
mod link;

use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir, create_dir_all, read_dir, read_to_string, remove_file},
    io::{self, stdout},
    path::{Path, PathBuf},
    process::Command,
};

use blake3::{hash, Hash};
use clap::{command, Parser};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
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

    /// Building release
    #[arg(long)]
    release: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let project_config_path = Path::new(&args.file);
    let project_path = project_config_path.parent().unwrap_or(Path::new("./"));

    if let Ok(mut project_config) = ProjectConfig::load(project_config_path) {
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
            SetForegroundColor(Color::parse_ansi("2;24;80;11").unwrap()),
            Print(r"                    |___/".to_string() + "\n"),
            ResetColor
        )?;

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

        if args.release {
            println!("Release mode.\n");
        } else {
            println!("Debug mode.\n");
        }

        let mut config = Config::load(project_path).unwrap_or_default();
        let mut hash_hashmap = if config.release != args.release {
            for entry in read_dir(&objects_dir_path)? {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if path.is_file() {
                        remove_file(path).ok();
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

        config.release = args.release;
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

        print!("{} file", files_to_compile.len());

        if files_to_compile.len() > 1 {
            print!("s");
        }

        print!(" to compile");

        if files_to_compile.len() > 0 {
            println!(" :");
        } else {
            println!(".");
        }

        let mut commands = vec![];

        for file in files_to_compile.iter() {
            println!(
                "  - {}",
                &file
                    .0
                    .strip_prefix(project_path)
                    .unwrap_or(file.0)
                    .to_string_lossy()
            );

            let mut command = Command::new(&project_config.compiler);

            if !args.release {
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

        for (file, mut command) in commands.drain(..) {
            if let Ok(exit_code) = command.wait() {
                if !exit_code.success() {
                    new_hash_hashmap.remove(file);
                }
            } else {
                new_hash_hashmap.remove(file);
            }
        }

        print!("{} file", files_to_link.len());

        if files_to_link.len() > 1 {
            print!("s");
        }

        print!(" to link");

        if files_to_link.len() > 0 {
            println!(" :");
        } else {
            println!(".");
        }

        for (main_file, file_to_link) in &files_to_link {
            println!(
                "  - {}",
                &main_file
                    .strip_prefix(project_path)
                    .unwrap_or(&main_file)
                    .to_string_lossy()
            );

            let mut command = Command::new(&project_config.compiler);

            if !args.release {
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
                .join(if args.release {
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

            output_file.set_extension("out");
            commands.push((
                main_file,
                command.arg("-o").arg(output_file).spawn().unwrap(),
            ));
        }

        for (file, mut command) in commands.drain(..) {
            if let Ok(exit_code) = command.wait() {
                if !exit_code.success() {
                    new_hash_hashmap.remove(file);
                }
            } else {
                new_hash_hashmap.remove(file);
            }
        }

        new_hash_hashmap.save(project_path)?;

        return Ok(());
    }

    eprintln!("No project config file found !");

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
