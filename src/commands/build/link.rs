use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
    sync::LazyLock,
};

use tokio::{process::Command, task::JoinSet};

use crate::{
    commands::build::{dependencies::Dependency, file::SourceFile},
    config::{self, Target, TargetType},
    helpers::{self, PathTarget},
    print_error,
};

static LIBRARIES_PATHS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut paths = vec![".".to_string()];

    for path in ["/usr/local/lib/", "/usr/lib/", "/lib/x86_64-linux-gnu/"] {
        if Path::new(path).is_dir() {
            paths.extend(["-Wl,-rpath".to_string(), path.to_string()]);
        }
    }

    paths.reverse();
    paths
});

pub async fn link(
    project_paths: &helpers::ProjectPaths,
    package_config: &config::Package,
    dependencies: &HashMap<String, Dependency>,
    targets_source_files: &Vec<(Target, HashSet<PathBuf>)>,
    source_files: &HashMap<PathBuf, SourceFile>,
    dependencies_lflags: &HashMap<String, Vec<String>>,
    release: bool,
) -> anyhow::Result<()> {
    if targets_source_files.is_empty() {
        return Ok(());
    }

    let mut commands: JoinSet<anyhow::Result<_>> = JoinSet::new();

    for (target, target_source_files) in targets_source_files {
        let object_files = target_source_files
            .into_iter()
            .map(|target_source_file| {
                package_config
                    .objects()
                    .target_release(release)
                    .join(source_files[target_source_file].hash.to_string())
            })
            .collect();

        let mut command = if target.target_type == TargetType::StaticLib {
            static_linking(project_paths, package_config, release, target, object_files).await?
        } else {
            dynamic_linking(
                project_paths,
                package_config,
                dependencies,
                release,
                target,
                object_files,
                &dependencies_lflags,
            )
            .await?
        };

        let target = target.clone();

        commands.spawn(async move { Ok((target, command.output().await?)) });
    }

    while let Some(command) = commands.join_next().await {
        let (target, output) = command??;

        if !output.status.success() {
            print_error(format!(
                "{}\n\n{}\n",
                project_paths.project_path.join(target.path).display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ))?;
        }
    }

    Ok(())
}

async fn static_linking(
    project_paths: &helpers::ProjectPaths,
    package_config: &config::Package,
    release: bool,
    target: &Target,
    object_files: Vec<PathBuf>,
) -> anyhow::Result<Command> {
    let mut command = package_config.archiver()?;

    command
        .current_dir(&project_paths.project_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .arg("rcs")
        .arg(
            package_config
                .binaries()
                .target_release(release)
                .join(target.binary_name()?),
        )
        .args(object_files);

    Ok(command)
}

async fn dynamic_linking(
    project_paths: &helpers::ProjectPaths,
    package_config: &config::Package,
    dependencies: &HashMap<String, Dependency>,
    release: bool,
    target: &Target,
    object_files: Vec<PathBuf>,
    dependencies_lflags: &HashMap<String, Vec<String>>,
) -> anyhow::Result<Command> {
    let lflags = target
        .import
        .iter()
        .flat_map(|import| dependencies.get(import))
        .fold(
            [
                dependencies_lflags
                    .get(&target.name()?)
                    .cloned()
                    .unwrap_or_default(),
                target.lflags.clone(),
                package_config.lflags.clone(),
            ]
            .concat(),
            |mut acc, dependency_config| {
                acc.extend(dependency_config.lflags.clone());
                acc.push("-L".to_string());
                acc.extend(
                    dependency_config
                        .directories
                        .iter()
                        .map(|directory| [directory.clone(), "-Wl,-rpath".to_string()])
                        .flatten(),
                );
                acc.extend(LIBRARIES_PATHS.clone());
                acc.extend(
                    dependency_config
                        .libraries
                        .iter()
                        .map(|library| format!("-l{library}")),
                );
                acc
            },
        );

    let mut command = package_config.linker()?;

    command
        .current_dir(&project_paths.project_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .arg("-fdiagnostics-color=always");

    if release {
        command.arg("-s");
    } else {
        command.arg("-g").arg("-Wall");
    };

    if target.target_type == TargetType::Dylib {
        command.arg("-shared");
    }

    command.args(lflags).args(object_files).arg("-o").arg(
        package_config
            .binaries()
            .target_release(release)
            .join(target.binary_name()?),
    );

    Ok(command)
}
