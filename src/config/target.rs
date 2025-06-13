use std::{env, path::PathBuf};

use anyhow::anyhow;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

use crate::config::require::RequireConfig;

#[serde_inline_default]
#[derive(Deserialize, Debug, Clone)]
pub struct Target {
    #[serde(default)]
    pub require: RequireConfig,
    name: Option<String>,
    pub path: PathBuf,
    #[serde(default)]
    pub import: Vec<String>,
    #[serde_inline_default(TargetType::Bin)]
    #[serde(rename = "type")]
    pub package_type: TargetType,
}

impl Target {
    pub fn name(&self) -> anyhow::Result<String> {
        self.name
            .clone()
            .or(self
                .path
                .file_stem()
                .map(|name| name.to_string_lossy().to_string()))
            .ok_or(anyhow!("can't create name from `{}`", self.path.display()))
    }

    pub fn binary_name(&self) -> anyhow::Result<String> {
        let name = self.name()?;

        Ok(match &self.package_type {
            TargetType::Bin => format!("{name}{}", env::consts::EXE_SUFFIX),
            TargetType::StaticLib => format!("{name}.a"),
            TargetType::Dylib => format!(
                "{}{name}.{}",
                env::consts::DLL_PREFIX,
                env::consts::DLL_EXTENSION
            ),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TargetType {
    Bin,
    StaticLib,
    #[serde(alias = "lib")]
    Dylib,
}
