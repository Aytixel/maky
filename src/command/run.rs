use std::{
    env,
    io::{self, stdout},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};

use crate::config::{LoadConfig, ProjectConfig};

use super::{build, get_project_path};

pub fn run(
    config_file: String,
    release: bool,
    rebuild: bool,
    file: PathBuf,
    args: Vec<String>,
) -> io::Result<()> {
    build(config_file.clone(), release, rebuild, true)?;

    let (project_path, project_config_path) = &get_project_path(&config_file);

    match ProjectConfig::load(project_config_path) {
        Ok(project_config) => {
            let mut output_file = project_config
                .binaries
                .join(if release {
                    Path::new("release")
                } else {
                    Path::new("debug")
                })
                .join(file);

            output_file.set_extension(env::consts::EXE_EXTENSION);

            let output_file_exist = project_path.join(&output_file).exists();

            execute!(
                stdout(),
                SetForegroundColor(if output_file_exist {
                    Color::DarkGreen
                } else {
                    Color::Red
                }),
                Print("     Running ".bold()),
                ResetColor,
                Print("`"),
                Print(output_file.to_string_lossy()),
                Print(
                    match args
                        .iter()
                        .cloned()
                        .reduce(|accumulator, arg| accumulator + " " + &arg)
                    {
                        Some(value) => " ".to_string() + &value,
                        None => "".to_string(),
                    }
                ),
                Print("`\n")
            )?;

            if output_file_exist {
                Command::new(output_file)
                    .current_dir(project_path)
                    .args(args)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .unwrap();
            }
        }
        Err(error) => ProjectConfig::handle_error(error, project_config_path)?,
    }

    Ok(())
}
