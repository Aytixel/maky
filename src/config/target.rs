use std::{env, path::Path};

use anyhow::anyhow;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

use crate::config::{replace_path_templates, require::Require};

#[serde_inline_default]
#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
    #[serde(default)]
    pub(in crate::config) require: Require,
    name: Option<String>,
    #[serde_inline_default("src/main.c".to_string())]
    pub path: String,
    #[serde(default)]
    pub import: Vec<String>,
    #[serde_inline_default(TargetType::Bin)]
    #[serde(rename = "type")]
    pub target_type: TargetType,

    #[serde(default)]
    pub lflags: Vec<String>,
}

impl Target {
    pub fn name(&self) -> anyhow::Result<String> {
        self.name
            .clone()
            .or(Path::new(&self.path)
                .file_stem()
                .map(|name| name.to_string_lossy().to_string()))
            .ok_or(anyhow!("can't create name from `{}`", self.path))
    }

    pub fn binary_name(&self) -> anyhow::Result<String> {
        let name = self.name()?;

        Ok(match &self.target_type {
            TargetType::Bin => format!("{name}{}", env::consts::EXE_SUFFIX),
            TargetType::StaticLib => format!("{}{name}.a", env::consts::DLL_PREFIX),
            TargetType::Dylib => format!(
                "{}{name}.{}",
                env::consts::DLL_PREFIX,
                env::consts::DLL_EXTENSION
            ),
        })
    }

    pub(in crate::config) fn apply_path_templates(mut self) -> Self {
        self.path = replace_path_templates(self.path);
        self
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TargetType {
    Bin,
    StaticLib,
    #[serde(alias = "lib")]
    Dylib,
}
