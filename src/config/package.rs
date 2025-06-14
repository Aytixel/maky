use std::path::{Path, PathBuf};

use anyhow::anyhow;
use semver::Version;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use serde_with::{formats::PreferOne, serde_as, OneOrMany};
use tokio::process::Command;
use which::which;

use crate::config::require::RequireConfig;

#[serde_as]
#[serde_inline_default]
#[derive(Deserialize, Debug, Clone)]
pub struct Package {
    #[serde(default)]
    pub require: RequireConfig,
    pub name: Option<String>,
    pub version: Option<Version>,

    /// C compiler config
    #[serde_inline_default(vec!["gcc".to_string(), "clang".to_string()])]
    #[serde(rename = "cc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    c_compilers: Vec<String>,
    #[serde(rename = "cstd")]
    c_standard: Option<String>,

    /// C++ compiler config
    #[serde_inline_default(vec!["g++".to_string(), "clang++".to_string()])]
    #[serde(rename = "cxx")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    cxx_compilers: Vec<String>,
    #[serde(rename = "cxxstd")]
    cxx_standard: Option<String>,

    /// Directories config
    #[serde_inline_default(Path::new("bin").to_path_buf())]
    #[serde(alias = "bin")]
    pub binaries: PathBuf,
    #[serde_inline_default(Path::new("obj").to_path_buf())]
    #[serde(alias = "obj")]
    pub objects: PathBuf,
    #[serde_inline_default(vec![Path::new("src").to_path_buf()])]
    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub sources: Vec<PathBuf>,
    #[serde_inline_default(vec![Path::new("include").to_path_buf()])]
    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub includes: Vec<PathBuf>,
}

impl Package {
    pub fn c_compiler(&self) -> anyhow::Result<Command> {
        let c_compiler = self
            .c_compilers
            .iter()
            .find_map(|c_compiler| which(c_compiler).ok())
            .ok_or(anyhow!(
                "can't find any c compiler: {}",
                self.c_compilers.join(", ")
            ))?;
        let mut command = Command::new(c_compiler);

        command.kill_on_drop(true);

        if let Some(c_standard) = &self.c_standard {
            command.arg(format!("-std={c_standard}"));
        }

        Ok(command)
    }

    pub fn cxx_compiler(&self) -> anyhow::Result<Command> {
        let cxx_compiler = self
            .cxx_compilers
            .iter()
            .find_map(|cxx_compiler| which(cxx_compiler).ok())
            .ok_or(anyhow!(
                "can't find any c compiler: {}",
                self.cxx_compilers.join(", ")
            ))?;
        let mut command = Command::new(cxx_compiler);

        command.kill_on_drop(true);

        if let Some(cxx_standard) = &self.cxx_standard {
            command.arg(format!("-std={cxx_standard}"));
        }

        Ok(command)
    }
}
