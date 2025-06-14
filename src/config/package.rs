use anyhow::anyhow;
use semver::Version;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use serde_with::{formats::PreferOne, serde_as, OneOrMany};
use tokio::process::Command;
use which::which;

use crate::config::{replace_path_templates, require::Require};

#[serde_as]
#[serde_inline_default]
#[derive(Deserialize, Debug, Clone)]
pub struct Package {
    #[serde(default)]
    pub(in crate::config) require: Require,
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
    #[serde_inline_default("bin".to_string())]
    #[serde(alias = "bin")]
    pub binaries: String,
    #[serde_inline_default("obj".to_string())]
    #[serde(alias = "obj")]
    pub objects: String,
    #[serde_inline_default(vec!["src".to_string()])]
    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub sources: Vec<String>,
    #[serde_inline_default(vec!["include".to_string()])]
    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub includes: Vec<String>,
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

    pub(in crate::config) fn apply_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    pub(in crate::config) fn apply_version(mut self, version: Option<Version>) -> Self {
        self.version = version;
        self
    }

    pub(in crate::config) fn apply_path_templates(mut self) -> Self {
        self.binaries = replace_path_templates(self.binaries);
        self.objects = replace_path_templates(self.objects);
        self.sources = self
            .sources
            .into_iter()
            .map(replace_path_templates)
            .collect();
        self.includes = self
            .includes
            .into_iter()
            .map(replace_path_templates)
            .collect();
        self
    }
}
