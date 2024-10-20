use std::{fs::read_to_string, io, path::Path};

use dependency::DependencyConfig;
use hashbrown::HashMap;
use lib::LibConfig;
use package::PackageConfig;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use specific::SpecificConfig;

pub mod dependency;
pub mod features;
pub mod lib;
pub mod package;
pub mod specific;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    pub package: Option<PackageConfig>,

    #[serde(default = "ProjectConfig::default_dependencies")]
    #[serde(alias = "deps")]
    pub dependencies: HashMap<String, DependencyConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "libs")]
    pub libraries: HashMap<String, LibConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "arch")]
    pub arch_specific: HashMap<String, SpecificConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "feat")]
    pub feature_specific: HashMap<String, SpecificConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "os", rename = "os-specific")]
    pub os_specific: HashMap<String, SpecificConfig>,
}

impl ProjectConfig {
    fn default_dependencies() -> HashMap<String, DependencyConfig> {
        HashMap::new()
    }

    fn default_hashmap<T>() -> HashMap<String, T> {
        HashMap::new()
    }

    pub fn load(file_path: &Path) -> io::Result<Self> {
        toml::from_str(&read_to_string(file_path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
    }
}
