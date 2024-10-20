mod compiling;
mod dependencies;
mod linking;

use std::{
    fs::{create_dir, create_dir_all, read, read_dir, remove_dir, remove_file},
    io::{stdout, Write},
    path::Path,
    time::Instant,
};

use blake3::hash;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use hashbrown::HashMap;

use crate::{
    config::{hash::LoadHash, ProjectConfig},
    file::{compile::compile, link::link, scan_dir},
};

use self::compiling::compiling;
use self::dependencies::dependencies;
use self::linking::linking;

use super::{add_mode_path, get_project_path};

#[derive(Clone, Copy)]
pub struct BuildFlags {
    pub release: bool,
    pub rebuild: bool,
    pub pretty: bool,
}

pub fn build(
    config_file: String,
    flags: &BuildFlags,
    stderr: &mut impl Write,
) -> anyhow::Result<bool> {
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

    let mut project_config = match ProjectConfig::load(project_config_path) {
        Ok(project_config) => project_config,
        Err(error) => {
            ProjectConfig::handle_error(error, project_config_path)?;

            return Ok(true);
        }
    };
    let Some(mut package_config) = project_config.package.clone() else {
        return Ok(false);
    };

    let dir_path = project_path.join("./.maky");
    if !dir_path.is_dir() {
        create_dir(dir_path)?;
    }

    let binaries_dir_path = project_path.join(&package_config.binaries);
    if !binaries_dir_path.is_dir() {
        create_dir(&binaries_dir_path)?;
    }

    for source in package_config.sources.iter() {
        let sources_dir_path = project_path.join(source);
        if !sources_dir_path.is_dir() {
            create_dir(sources_dir_path)?;
        }

        package_config.includes.push(source.clone());
    }

    let objects_dir_path =
        add_mode_path(&project_path.join(&package_config.objects), flags.release);
    if !objects_dir_path.is_dir() {
        create_dir_all(&objects_dir_path)?;
    }

    for library in project_config.libraries.values() {
        package_config.includes.extend_from_slice(&library.includes);
    }

    package_config
        .includes
        .push(Path::new(".maky/include").to_path_buf());

    if cfg!(target_os = "linux") {
        package_config.includes.push("/usr/include".into());
    }

    project_config.package = Some(package_config);

    let need_rebuild = dependencies(project_path, &mut project_config, flags, stderr)?;
    let mut hash_hashmap = if flags.rebuild || need_rebuild {
        remove_objects(&objects_dir_path)?;

        HashMap::new()
    } else {
        HashMap::load(project_path, flags.release).unwrap_or_default()
    };
    let mut new_hash_hashmap = HashMap::new();
    let mut main_hashmap = HashMap::new();
    let mut lib_hashmap = HashMap::new();
    let mut import_hashmap = HashMap::new();
    let mut h_h_link = HashMap::new();
    let mut h_c_link = HashMap::new();
    let mut c_h_link = HashMap::new();

    if let Some(package_config) = &project_config.package {
        for source in package_config.sources.iter() {
            scan_dir(
                project_path,
                &project_config,
                &project_path.join(source),
                &mut main_hashmap,
                &mut lib_hashmap,
                &mut import_hashmap,
                &mut h_h_link,
                &mut h_c_link,
                &mut c_h_link,
                &mut new_hash_hashmap,
            )?;
        }
    }

    let project_config_hash = hash(&read(project_config_path)?);

    new_hash_hashmap.insert(project_config_path.to_owned(), project_config_hash);

    if hash_hashmap
        .get(project_config_path)
        .map(|hash| hash != &project_config_hash)
        .unwrap_or(true)
    {
        remove_objects(&objects_dir_path)?;

        hash_hashmap.clear();
    }

    let is_rebuilding = new_hash_hashmap != hash_hashmap;

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
        flags,
        stderr,
    )?;

    let files_to_link = link(
        project_path,
        &project_config,
        &main_hashmap,
        &lib_hashmap,
        &files_to_compile,
        &h_c_link,
        &c_h_link,
    )?;

    linking(
        project_path,
        &project_config,
        &import_hashmap,
        &files_to_link,
        new_hash_hashmap,
        flags,
        stderr,
    )?;

    if flags.pretty {
        execute!(
            stdout(),
            SetForegroundColor(Color::DarkGreen),
            Print("    Finished ".bold()),
            ResetColor,
            Print(if flags.release {
                "`release` profile [optimized]"
            } else {
                "`dev` profile [unoptimized + debuginfo]"
            }),
            Print(format!(" target(s) in {:.2?}\n", time.elapsed()))
        )?;
    }

    Ok(is_rebuilding)
}

fn remove_objects(objects_dir_path: &Path) -> anyhow::Result<()> {
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

    Ok(())
}
