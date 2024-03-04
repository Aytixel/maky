use std::{
    env,
    fs::{create_dir_all, hard_link, remove_dir_all, remove_file},
    io::{self, stderr, Read},
    path::Path,
    process::{Command, Stdio},
};

use ahash::AHashMap;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use git2::{Error, Repository};
use kdam::{tqdm, BarExt, Column, RichProgress, Spinner};
use parse_git_url::GitUrl;

use crate::{
    command::{add_mode_path, get_project_path},
    config::{DependencyConfig, LibConfig, LoadConfig, ProjectConfig},
    file::scan_dir_dependency,
};

use super::BuildFlags;

pub fn dependencies(
    project_path: &Path,
    project_config: &mut ProjectConfig,
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

    let mut errors = Vec::new();
    let mut commands = Vec::new();
    let project_dependencies_path = project_path.join(".maky/dependencies");

    create_dir_all(&project_dependencies_path)?;

    for (dependency_name, dependency_config) in project_config.dependencies.iter() {
        let dependency_path = match dependency_config {
            DependencyConfig::Local { path } => project_path.join(path),
            DependencyConfig::Git { git, rev } => {
                let mut git_errors = Vec::new();
                let git_url = GitUrl::parse(git)
                    .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
                let project_dependency_path = project_dependencies_path.join(&git_url.name);

                if !project_dependency_path.is_dir() {
                    if let Err(error) = Repository::clone_recurse(git, &project_dependency_path) {
                        git_errors.push(error.to_string());
                    }
                }

                fn pull(project_dependency_path: &Path, rev: &Option<String>) -> Result<(), Error> {
                    let repository = Repository::open(&project_dependency_path)?;
                    let mut remote = repository.find_remote("origin")?;

                    remote.fetch(&[] as &[&str], None, None)?;

                    let rev = rev.clone().unwrap_or(
                        remote
                            .default_branch()
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string(),
                    );
                    let (object, reference) = repository.revparse_ext(&rev)?;

                    repository.checkout_tree(&object, None)?;

                    match reference {
                        Some(mut reference) => {
                            let fetch_head = repository.find_reference("FETCH_HEAD")?;
                            let fetch_commit =
                                repository.reference_to_annotated_commit(&fetch_head)?;
                            let analysis = repository.merge_analysis(&[&fetch_commit])?;

                            if analysis.0.is_up_to_date() {
                                repository.set_head(reference.name().unwrap())
                            } else if analysis.0.is_fast_forward() {
                                reference.set_target(fetch_commit.id(), "Fast-Forward")?;
                                repository.set_head(reference.name().unwrap())?;
                                repository.checkout_head(Some(
                                    git2::build::CheckoutBuilder::default().force(),
                                ))
                            } else {
                                Err(Error::from_str("Fast-forward only!"))
                            }
                        }
                        None => repository.set_head_detached(object.id()),
                    }
                }

                if let Err(error) = pull(&project_dependency_path, rev) {
                    git_errors.push(error.to_string());
                }

                if !git_errors.is_empty() {
                    errors.push((
                        dependency_name.clone(),
                        format!("{}", git_errors.join("\n")),
                    ));
                }

                project_dependency_path
            }
        };
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

    let binaries_path = add_mode_path(&project_config.binaries, flags.release);
    let project_binaries_path = project_path.join(&binaries_path);

    remove_dir_all(project_path.join(".maky/include")).ok();

    for (dependency_name, dependency_path, mut command) in commands.into_iter() {
        if let Some(dependencies_progress_bar) = &mut dependencies_progress_bar_option {
            dependencies_progress_bar.columns[2] =
                Column::Text("[bold blue]".to_string() + &dependency_name);
            dependencies_progress_bar.update(1)?;
        }

        let project_include_path = project_path
            .join(".maky/include/deps")
            .join(dependency_name);

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
                                hard_link(
                                    &h_file,
                                    project_include_path
                                        .join(h_file.strip_prefix(&include_path).unwrap()),
                                )?;
                            }
                        }
                    }

                    for entry in add_mode_path(
                        &dependency_path.join(dependency_config.binaries),
                        flags.release,
                    )
                    .read_dir()?
                    {
                        if let Ok(entry) = entry {
                            let path = entry.path();

                            if let Some(true) = path.file_name().map(|file_name| {
                                file_name
                                    .to_string_lossy()
                                    .contains(env::consts::DLL_SUFFIX)
                            }) {
                                create_dir_all(&project_binaries_path)?;

                                let link = project_binaries_path.join(path.file_name().unwrap());

                                remove_file(&link).ok();
                                hard_link(&path, link)?;

                                let lib_name = path.file_stem().unwrap().to_string_lossy();
                                let lib_name = lib_name.strip_prefix("lib").unwrap_or(&lib_name);
                                let lib_config = LibConfig {
                                    library: vec![lib_name.to_string()],
                                    directories: vec![binaries_path.clone()],
                                    includes: Vec::new(),
                                    pkg_config: AHashMap::new(),
                                };

                                project_config
                                    .libraries
                                    .insert(format!("{dependency_name}/{lib_name}"), lib_config);
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
            let error = error.trim();

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
                    Print(dependency_name),
                    Print("\n\n")
                )?;

                eprintln!("{error}\n");
            }
        }
    }

    Ok(())
}
