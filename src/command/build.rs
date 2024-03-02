mod compiling;
mod dependencies;
mod linking;

use std::{
    fs::{create_dir, create_dir_all, read_dir, remove_dir, remove_file},
    io::{self, stdout},
    path::Path,
    time::Instant,
};

use ahash::AHashMap;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};

use crate::{
    config::{LoadConfig, ProjectConfig},
    file::{compile::compile, link::link, scan_dir},
};

use self::compiling::compiling;
use self::dependencies::dependencies;
use self::linking::linking;

use super::{add_mode_path, get_project_path};

pub struct BuildFlags {
    pub release: bool,
    pub rebuild: bool,
    pub pretty: bool,
}

pub fn build(config_file: String, flags: BuildFlags) -> io::Result<()> {
    let (project_path, project_config_path) = &get_project_path(&config_file);
    let time = Instant::now();

    if flags.pretty {
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
            if flags.release {
                Print(r"Release             ".bold())
            } else {
                Print(r"Dev                 ".bold())
            },
            SetForegroundColor(Color::parse_ansi("2;24;80;11").unwrap()),
            Print(r"|___/".to_string() + "\n\n"),
            ResetColor
        )?;
    }

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

            let objects_dir_path =
                add_mode_path(&project_path.join(&project_config.objects), flags.release);
            if !objects_dir_path.is_dir() {
                create_dir_all(&objects_dir_path)?;
            }

            for library in project_config.libraries.values() {
                project_config.includes.extend_from_slice(&library.includes);
            }

            project_config
                .includes
                .push(Path::new(".maky/include").to_path_buf());

            if cfg!(target_os = "linux") {
                project_config.includes.push("/usr/include".into());
            }

            dependencies(project_path, &mut project_config, &flags)?;

            let mut hash_hashmap = if flags.rebuild {
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
            let mut main_vec = Vec::new();
            let mut lib_hashmap = AHashMap::new();
            let mut h_h_link = AHashMap::new();
            let mut h_c_link = AHashMap::new();
            let mut c_h_link = AHashMap::new();

            for source in project_config.sources.iter() {
                scan_dir(
                    project_path,
                    &project_config,
                    &project_path.join(source),
                    &mut main_vec,
                    &mut lib_hashmap,
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

            compiling(
                project_path,
                &project_config,
                &files_to_compile,
                &mut new_hash_hashmap,
                &flags,
            )?;

            let files_to_link = link(
                project_path,
                &project_config,
                &main_vec,
                &lib_hashmap,
                &files_to_compile,
                &h_c_link,
                &c_h_link,
            )?;

            linking(
                project_path,
                &project_config,
                &files_to_link,
                new_hash_hashmap,
                &flags,
            )?;

            if flags.pretty {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::DarkGreen),
                    Print("    Finished ".bold()),
                    ResetColor,
                    Print(if flags.release {
                        "release [optimized]"
                    } else {
                        "dev [unoptimized + debuginfo]"
                    }),
                    Print(format!(" target(s) in {:.2?}\n", time.elapsed()))
                )?;
            }
        }
        Err(error) => ProjectConfig::handle_error(error, project_config_path)?,
    }

    Ok(())
}
