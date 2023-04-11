mod compile;
mod config;
mod link;

use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir, read_dir, read_to_string},
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
    config::{load_hash_file, save_hash_file, Config, LibConfig},
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
    let config_path = Path::new(&args.file);
    let config_dir_path = config_path.parent().unwrap_or(Path::new("./"));

    if let Ok(config_file) = read_to_string(config_path) {
        let mut config: Config = toml::from_str(&config_file).unwrap();

        execute!(
            stdout(),
            SetForegroundColor(Color::parse_ansi("2;118;199;56").unwrap()),
            Print(r"              _          ".to_string() + "\n"),
            Print(r"  /\/\   __ _| | ___   _ ".to_string() + "\n"),
            SetForegroundColor(Color::parse_ansi("2;101;171;48").unwrap()),
            Print(r" /    \ / _` | |/ / | | |".to_string() + "\n"),
            Print(r"/ /\/\ \ (_| |   <| |_| |".to_string() + "\n"),
            SetForegroundColor(Color::parse_ansi("2;85;143;40").unwrap()),
            Print(r"\/    \/\__,_|_|\_\\__, |".to_string() + "\n"),
            Print(r"                    |___/".to_string() + "\n"),
            ResetColor
        )?;

        let dir_path = config_dir_path.join("./.maky");
        if !dir_path.is_dir() {
            create_dir(dir_path)?;
        }

        let binaries_dir_path = config_dir_path.join(config.binaries.clone());
        if !binaries_dir_path.is_dir() {
            create_dir(binaries_dir_path.clone())?;
        }

        for source in config.sources.iter() {
            let sources_dir_path = config_dir_path.join(source);

            if !sources_dir_path.is_dir() {
                create_dir(sources_dir_path)?;
            }

            config.includes.push(config_dir_path.join(source));
        }

        let objects_dir_path = config_dir_path.join(config.objects.clone());
        if !objects_dir_path.is_dir() {
            create_dir(objects_dir_path.clone())?;
        }

        config.includes.push(config_dir_path.join("/usr/include"));

        println!("{:#?}", args);
        println!("{:#?}", config);

        let mut hash_hashmap = load_hash_file(config_dir_path);
        let mut new_hash_hashmap = HashMap::new();
        let mut main_hashset = HashSet::new();
        let mut h_h_link = HashMap::new();
        let mut h_c_link = HashMap::new();
        let mut c_h_link = HashMap::new();

        for source in config.sources.iter() {
            scan_dir(
                &config,
                &config_dir_path.join(source),
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
        )?;

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
                    .strip_prefix(config_dir_path)
                    .unwrap_or(file.0)
                    .to_string_lossy()
            );

            let mut command = Command::new(&config.compiler);

            if !args.release {
                command.arg("-g").arg("-Wall");
            }

            for include in config.includes.iter() {
                command.arg("-I").arg(include);
            }

            commands.push((
                file.0.to_path_buf(),
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
            &config,
            &main_hashset,
            &files_to_compile,
            &h_c_link,
            &c_h_link,
        );

        for (file, mut command) in commands.drain(..) {
            if let Ok(exit_code) = command.wait() {
                if !exit_code.success() {
                    new_hash_hashmap.remove(&file);
                }
            } else {
                new_hash_hashmap.remove(&file);
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

        for (main_file, file_to_link) in files_to_link {
            println!(
                "  - {}",
                &main_file
                    .strip_prefix(config_dir_path)
                    .unwrap_or(&main_file)
                    .to_string_lossy()
            );

            let mut command = Command::new(&config.compiler);

            if !args.release {
                command.arg("-g").arg("-Wall");
            }

            for c_file in file_to_link {
                command.arg(objects_dir_path.join(new_hash_hashmap[&c_file].to_hex().as_str()));
            }

            for lib in {
                let mut libs: Vec<&LibConfig> = config.libraries.values().collect();

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

            let mut output_file = binaries_dir_path.join(main_file.file_stem().unwrap());

            output_file.set_extension("out");
            commands.push((
                main_file,
                command.arg("-o").arg(output_file).spawn().unwrap(),
            ));
        }

        for (file, mut command) in commands.drain(..) {
            if let Ok(exit_code) = command.wait() {
                if !exit_code.success() {
                    new_hash_hashmap.remove(&file);
                }
            } else {
                new_hash_hashmap.remove(&file);
            }
        }

        save_hash_file(config_dir_path, &new_hash_hashmap)?;

        return Ok(());
    }

    eprintln!("No config file found !");

    return Ok(());
}

fn scan_dir(
    config: &Config,
    dir_path: &Path,
    main_hashset: &mut HashSet<PathBuf>,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    for entry in read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let extension = path.extension().unwrap_or_default();

            if extension == "c" || extension == "h" {
                let code = read_to_string(&path).unwrap();
                let includes = get_includes(&path, config.includes.clone(), &code);

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
                config,
                &path,
                main_hashset,
                h_h_link,
                h_c_link,
                c_h_link,
                hash_hashmap,
            )?;
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

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn getting_includes() {
        println!(
            "{:?}",
            get_includes(
                Path::new("window/window.c"),
                vec![
                    Path::new("./data/src/").to_path_buf(),
                    Path::new("/usr/include").to_path_buf()
                ],
                &read_to_string(Path::new("./data/src/window/window.c")).unwrap()
            )
        );
    }
}
