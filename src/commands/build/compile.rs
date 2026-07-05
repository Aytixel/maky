use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    process::Stdio,
};

use tokio::task::JoinSet;

use crate::{
    commands::build::{dependencies::Dependency, file::SourceFile},
    config,
    file::Language,
    helpers::{self, AsStrVec, PathTarget},
    print_error,
};

pub async fn compile(
    project_paths: &helpers::ProjectPaths,
    package_config: &config::Package,
    dependencies: &HashMap<String, Dependency>,
    updated_source_files: &HashSet<PathBuf>,
    source_files: &HashMap<PathBuf, SourceFile>,
    release: bool,
) -> anyhow::Result<()> {
    if updated_source_files.is_empty() {
        return Ok(());
    }

    let (includes, cflags, defines) = dependencies.values().fold(
        (
            package_config.includes.as_str_vec(),
            package_config.cflags.as_str_vec(),
            package_config.defines.as_str_vec(),
        ),
        |mut acc, dependency_config| {
            acc.0.extend(dependency_config.includes.as_str_vec());
            acc.1.extend(dependency_config.cflags.as_str_vec());
            acc.2.extend(dependency_config.defines.as_str_vec());
            acc
        },
    );
    let includes: Vec<&str> = includes
        .iter()
        .flat_map(|include| ["-I", include])
        .collect();
    let defines: Vec<&str> = defines.iter().flat_map(|define| ["-D", define]).collect();

    let mut commands: JoinSet<anyhow::Result<_>> = JoinSet::new();

    for updated_source_file in updated_source_files {
        let updated_source_file = updated_source_file.clone();
        let mut command = match source_files[&updated_source_file].language {
            Language::C => package_config.c_compiler(),
            Language::Cpp => package_config.cxx_compiler(),
        }?;

        command
            .current_dir(&project_paths.project_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .arg("-fdiagnostics-color=always")
            .arg("-fpic");

        if release {
            command.arg("-O3");
        } else {
            command.arg("-O0").arg("-g").arg("-Wall");
        };

        let object_file = package_config
            .objects()
            .target_release(release)
            .join(source_files[&updated_source_file].hash.to_string());

        command
            .args(&includes)
            .args(&cflags)
            .args(&defines)
            .arg("-c")
            .arg(&updated_source_file)
            .arg("-o")
            .arg(&object_file);

        commands.spawn(async move { Ok((updated_source_file, command.output().await?)) });
    }

    while let Some(command) = commands.join_next().await {
        let (source_file, output) = command??;

        if !output.status.success() {
            print_error(format!(
                "{}\n\n{}\n",
                project_paths.project_path.join(source_file).display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ))?;
        }
    }

    Ok(())
}
