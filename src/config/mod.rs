use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Formatter},
};

use anyhow::anyhow;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

pub use crate::config::{
    dependency::{Dependency, TypedDependency},
    package::Package,
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
    pub fn values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a T> + 'a> {
        match self {
            VecOrValue::Value(v) => Box::new(vec![v].into_iter()),
            VecOrValue::Vec(v) => Box::new(v.iter()),
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
        Debug::fmt(&self.values().collect::<Vec<_>>(), f)
    }
}

#[serde_inline_default]
#[derive(Deserialize, Debug)]
pub struct Project {
    package: VecOrValue<Package>,
    #[serde(default)]
    dependencies: HashMap<String, VecOrValue<Dependency>>,
    #[serde(rename = "bin", default)]
    binaries: Vec<Target>,
}

impl Project {
    pub fn package(&self) -> anyhow::Result<Package> {
        let mut name = None;
        let mut version = None;

        // Search the first name and version declared since we allow to have
        // multiple system dependent package definition, but a package should
        // have one and only name and version
        for package in self.package.values() {
            if name.is_none() && package.name.is_some() {
                name = package.name.clone();
            }
            if version.is_none() && package.version.is_some() {
                version = package.version.clone();
            }
            if name.is_some() && version.is_some() {
                break;
            }
        }

        self.package
            .values()
            .find(|package| package.require.has_requirements())
            .cloned()
            .map(|package| {
                package
                    .apply_name(name)
                    .apply_version(version)
                    .apply_path_templates()
            })
            .ok_or(anyhow!("no feating package configuration found"))
    }

    pub fn dependencies(&self) -> HashMap<String, Vec<Dependency>> {
        self.dependencies
            .iter()
            .map(|(name, dependency_targets)| {
                (
                    name.clone(),
                    dependency_targets
                        .values()
                        .into_iter()
                        .filter(|dependency| dependency.require.has_requirements())
                        .cloned()
                        .map(Dependency::apply_path_templates)
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, dependency_targets)| !dependency_targets.is_empty())
            .collect()
    }

    pub fn binaries(&self) -> Vec<Target> {
        self.binaries
            .iter()
            .filter(|binary| binary.require.has_requirements())
            .cloned()
            .map(Target::apply_path_templates)
            .collect()
    }
}

fn replace_path_templates<T: AsRef<str>>(string: T) -> String {
    string
        .as_ref()
        .replace("{os}", env::consts::OS)
        .replace("{family}", env::consts::FAMILY)
        .replace("{arch}", env::consts::ARCH)
}
