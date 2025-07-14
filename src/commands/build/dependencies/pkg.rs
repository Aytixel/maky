use std::{path::Path, process::Stdio};

use anyhow::anyhow;
use semver::{Version, VersionReq};
use tokio::{join, process::Command};

use crate::{commands::build::dependencies::Dependency, config::VecOrValue};

pub async fn get_pkg_dependency(
    pkg_config_command: &Path,
    paths: VecOrValue<String>,
    pkg: Vec<(String, VersionReq)>,
) -> anyhow::Result<Dependency> {
    let libraries_requirements = get_libraries_requirements(pkg);
    let paths: Vec<_> = paths
        .values()
        .map(|path| format!("--with-path={path}"))
        .collect();
    let compiler_command = Command::new(pkg_config_command)
        .arg("--cflags")
        .args(&paths)
        .args(&libraries_requirements)
        .stdin(Stdio::null())
        .output();
    let linker_command = Command::new(pkg_config_command)
        .arg("--libs")
        .args(paths)
        .args(libraries_requirements)
        .stdin(Stdio::null())
        .output();

    let (compiler_command, linker_command) = join!(compiler_command, linker_command);
    let (compiler_command, linker_command) = (compiler_command?, linker_command?);

    let cflags = if compiler_command.status.success() {
        String::from_utf8_lossy(&compiler_command.stdout)
    } else {
        return Err(anyhow!(
            "{}",
            String::from_utf8_lossy(&compiler_command.stderr)
        ));
    };

    let lflags = if linker_command.status.success() {
        String::from_utf8_lossy(&linker_command.stdout)
    } else {
        return Err(anyhow!(
            "{}",
            String::from_utf8_lossy(&linker_command.stderr)
        ));
    };

    Ok(parse_flags(cflags.trim_end(), lflags.trim_end()))
}

fn get_libraries_requirements(pkg: Vec<(String, VersionReq)>) -> Vec<String> {
    let mut library_requirements = Vec::new();

    for (library_name, version_requirements) in pkg {
        if version_requirements.comparators.is_empty() {
            library_requirements.push(library_name.clone());
            continue;
        }

        for comparator in version_requirements.comparators {
            let mut version = Version::new(
                comparator.major,
                comparator.minor.unwrap_or_default(),
                comparator.patch.unwrap_or_default(),
            );
            version.pre = comparator.pre.clone();

            library_requirements.push(match comparator.op {
                semver::Op::Exact => format!("{library_name} = {version}"),
                semver::Op::Greater => format!("{library_name} > {version}"),
                semver::Op::GreaterEq => format!("{library_name} >= {version}"),
                semver::Op::Less => format!("{library_name} < {version}"),
                semver::Op::LessEq => format!("{library_name} <= {version}"),
                semver::Op::Tilde => {
                    let mut max_version = Version::new(
                        comparator.major,
                        comparator.minor.unwrap_or_default() + 1,
                        0,
                    );
                    max_version.pre = comparator.pre;

                    format!("{library_name} >= {version} {library_name} < {max_version}")
                }
                semver::Op::Caret => {
                    let mut max_version = if version.major == 0 {
                        Version::new(
                            comparator.major,
                            comparator.minor.unwrap_or_default() + 1,
                            0,
                        )
                    } else {
                        Version::new(comparator.major + 1, 0, 0)
                    };
                    max_version.pre = comparator.pre;

                    format!("{library_name} >= {version} {library_name} < {max_version}")
                }
                semver::Op::Wildcard => library_name.clone(),
                _ => library_name.clone(),
            });
        }
    }

    library_requirements
}

fn parse_flags(cflags: &str, lflags: &str) -> Dependency {
    let mut dependency = Dependency::default();

    for token in split_flags(cflags) {
        if let Some(token) = token.strip_prefix("-I") {
            dependency.includes.push(token.to_string());
        } else if let Some(token) = token.strip_prefix("-D") {
            dependency.defines.push(token.to_string());
        } else {
            dependency.cflags.push(token);
        }
    }

    for token in split_flags(lflags) {
        if let Some(token) = token.strip_prefix("-L") {
            dependency.directories.push(token.to_string());
        } else if let Some(token) = token.strip_prefix("-l") {
            dependency.libraries.push(token.to_string());
        } else {
            dependency.lflags.push(token);
        }
    }

    dependency
}

fn split_flags(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double_quotes => {
                in_single_quotes = !in_single_quotes;
            }
            '"' if !in_single_quotes => {
                in_double_quotes = !in_double_quotes;
            }
            c if c.is_whitespace() && !in_single_quotes && !in_double_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
