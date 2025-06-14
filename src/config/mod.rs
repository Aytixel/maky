use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};

use anyhow::anyhow;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

pub use crate::config::package::Package;
use crate::config::{
    dependency::{DependenciesConfig, DependencyConfig},
    target::Target,
};

mod dependency;
mod package;
mod require;
mod target;

// The order of variants is important and should not be changed
#[derive(Deserialize, Clone)]
#[serde(untagged)]
enum VecOrValue<T> {
    Vec(Vec<T>),
    Value(T),
}

impl<T> VecOrValue<T> {
    pub fn values(&self) -> Vec<&T> {
        match self {
            VecOrValue::Value(v) => vec![&v],
            VecOrValue::Vec(v) => v.iter().collect(),
        }
    }
}

impl<T> Default for VecOrValue<T> {
    fn default() -> Self {
        VecOrValue::Vec(Vec::new())
    }
}

impl<T: Debug> Debug for VecOrValue<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.values(), f)
    }
}

#[serde_inline_default]
#[derive(Deserialize, Debug)]
pub struct Project {
    package: VecOrValue<Package>,
    #[serde(default)]
    dependencies: DependenciesConfig,
    #[serde(rename = "bin", default)]
    binaries: Vec<Target>,
}

impl Project {
    pub fn package(&self) -> anyhow::Result<Package> {
        let packages = self.package.values();
        let mut name = None;
        let mut version = None;

        for package in &packages {
            if name.is_none() && package.name.is_some() {
                name = package.name.clone();
            }
            if version.is_none() && package.version.is_some() {
                version = package.version.clone();
            }
        }

        packages
            .into_iter()
            .cloned()
            .find(|package| package.require.has_requirements())
            .map(|mut package| {
                package.name = name;
                package.version = version;
                package
            })
            .ok_or(anyhow!("no feating package configuration found"))
    }

    pub fn dependencies(&self) -> HashMap<&str, Vec<&DependencyConfig>> {
        self.dependencies
            .iter()
            .map(|(name, dependency_targets)| {
                (
                    name.as_str(),
                    dependency_targets
                        .values()
                        .into_iter()
                        .filter(|dependency| dependency.require.has_requirements())
                        .collect(),
                )
            })
            .collect()
    }

    pub fn binaries(&self) -> Vec<&Target> {
        self.binaries
            .iter()
            .filter(|binary| binary.require.has_requirements())
            .collect()
    }
}
