use std::path::{Path, PathBuf};

use anyhow::anyhow;
use semver::Version;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;
use serde_with::{formats::PreferOne, serde_as, OneOrMany};
use tokio::process::Command;
use which::which;

use crate::{
    config::{replace_path_templates, require::Require},
    helpers::PathTarget,
};

#[serde_as]
#[serde_inline_default]
#[derive(Deserialize, Debug, Clone)]
pub struct Package {
    #[serde(default)]
    pub(in crate::config) require: Require,
    pub name: Option<String>,
    pub version: Option<Version>,

    /// C compiler config
    #[serde_inline_default(vec!["clang".to_string(), "gcc".to_string()])]
    #[serde(rename = "cc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    c_compilers: Vec<String>,
    #[serde(rename = "cstd")]
    c_standard: Option<String>,

    /// C++ compiler config
    #[serde_inline_default(vec!["clang++".to_string(), "g++".to_string()])]
    #[serde(rename = "cxx")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    cxx_compilers: Vec<String>,
    #[serde(rename = "cxxstd")]
    cxx_standard: Option<String>,

    /// Compiler options
    #[serde(default)]
    pub defines: Vec<String>,
    #[serde(default)]
    pub cflags: Vec<String>,

    /// Linker config
    #[serde_inline_default(vec!["mold".to_string(), "lld".to_string(), "gold".to_string(), "ld".to_string()])]
    #[serde(rename = "ld")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    linkers: Vec<String>,
    #[serde_inline_default(vec!["llvm-ar".to_string(), "gcc-ar".to_string(), "ar".to_string()])]
    #[serde(rename = "ar")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    archivers: Vec<String>,

    /// Linker options
    #[serde(default)]
    pub lflags: Vec<String>,

    /// Directories config
    #[serde_inline_default("bin".to_string())]
    #[serde(alias = "bin")]
    binaries: String,
    #[serde_inline_default("obj".to_string())]
    #[serde(alias = "obj")]
    objects: String,
    #[serde_inline_default(vec!["src".to_string()])]
    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub sources: Vec<String>,
    #[serde_inline_default(vec!["include".to_string(), "src".to_string()])]
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
                "can't find any cpp compiler: {}",
                self.cxx_compilers.join(", ")
            ))?;
        let mut command = Command::new(cxx_compiler);

        command.kill_on_drop(true);

        if let Some(cxx_standard) = &self.cxx_standard {
            command.arg(format!("-std={cxx_standard}"));
        }

        Ok(command)
    }

    pub fn linker(&self) -> anyhow::Result<Command> {
        let mut command = self.cxx_compiler().or_else(|_| self.c_compiler())?;
        let linker = self
            .linkers
            .iter()
            .find_map(|linker| which(linker).is_ok().then_some(linker))
            .ok_or(anyhow!(
                "can't find any linker: {}",
                self.linkers.join(", ")
            ))?;

        command.arg(format!("-fuse-ld={linker}"));

        Ok(command)
    }

    pub fn archiver(&self) -> anyhow::Result<Command> {
        let archiver = self
            .archivers
            .iter()
            .find_map(|archiver| which(archiver).ok())
            .ok_or(anyhow!(
                "can't find any archiver: {}",
                self.archivers.join(", ")
            ))?;
        let mut command = Command::new(archiver);

        command.kill_on_drop(true);

        Ok(command)
    }

    pub fn binaries(&self, release: bool) -> PathBuf {
        Path::new(&self.binaries).target_release(release)
    }

    pub fn objects(&self, release: bool) -> PathBuf {
        Path::new(&self.objects).target_release(release)
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
