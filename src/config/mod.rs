use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Formatter},
    iter,
    path::Path,
};

use anyhow::anyhow;
use serde::Deserialize;
use serde_inline_default::serde_inline_default;

pub use crate::config::{
    dependency::{Dependency, MakyPathDependency, TypedDependency},
    package::Package,
    target::{Target, TargetType},
};

mod dependency;
mod package;
mod require;
mod target;

// The order of variants is important and should not be changed
#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum VecOrValue<T> {
    Vec(Vec<T>),
    Value(T),
}

impl<T> VecOrValue<T> {
    pub fn values<'a>(&'a self) -> Box<dyn Iterator<Item = &'a T> + 'a> {
        match self {
            VecOrValue::Value(v) => Box::new(iter::once(v)),
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

impl<A> FromIterator<A> for VecOrValue<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self {
        Self::Vec(iter.into_iter().collect())
    }
}

#[serde_inline_default]
#[derive(Deserialize, Debug, Clone)]
pub struct Project {
    package: VecOrValue<Package>,
    #[serde(default)]
    dependencies: HashMap<String, VecOrValue<Dependency>>,
    #[serde(rename = "bin", default)]
    binaries: Vec<Target>,
    #[serde(rename = "example", default)]
    examples: Vec<Target>,
    #[serde(rename = "test", default)]
    tests: Vec<Target>,
    #[serde(rename = "bench", default)]
    benchmarks: Vec<Target>,
}

impl Project {
    pub fn package(&self, project_path: &Path) -> anyhow::Result<Package> {
        let mut packages: Vec<_> = self.package.values().collect();
        let mut name = None;
        let mut version = None;

        // Search the first name and version declared since we allow to have
        // multiple system dependent package definition, but a package should
        // have one and only name and version
        for package in &packages {
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

        if name.is_none() {
            name = Some(
                project_path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
            );
        }

        packages.sort_by_key(|package| &package.require);
        packages
            .into_iter()
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

    pub fn examples(&self) -> Vec<Target> {
        self.examples
            .iter()
            .filter(|example| example.require.has_requirements())
            .cloned()
            .map(Target::apply_path_templates)
            .collect()
    }

    pub fn tests(&self) -> Vec<Target> {
        self.tests
            .iter()
            .filter(|test| test.require.has_requirements())
            .cloned()
            .map(Target::apply_path_templates)
            .collect()
    }

    pub fn benchmarks(&self) -> Vec<Target> {
        self.benchmarks
            .iter()
            .filter(|benchmark| benchmark.require.has_requirements())
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
