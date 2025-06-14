use std::{collections::HashMap, path::PathBuf};

use semver::VersionReq;
use serde::Deserialize;
use serde_with::{formats::PreferOne, serde_as, OneOrMany};

use crate::config::{require::RequireConfig, VecOrValue};

pub type DependenciesConfig = HashMap<String, VecOrValue<DependencyConfig>>;

#[derive(Deserialize, Debug)]
pub struct DependencyConfig {
    #[serde(default)]
    pub require: RequireConfig,
    #[serde(flatten)]
    pub dependency: TypedDependencyConfig,
}

#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum TypedDependencyConfig {
    MakyLocal {
        #[serde(default)]
        version: Option<VersionReq>,
        path: PathBuf,
    },
    MakyGit {
        #[serde(default)]
        version: Option<VersionReq>,
        git: String,
        rev: Option<String>,
    },
    Local {
        #[serde(alias = "dir", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        directories: Vec<PathBuf>,
        #[serde(alias = "inc", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        includes: Vec<PathBuf>,
        #[serde(alias = "lib", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        library: Vec<String>,
    },
    Pkg {
        pkg: HashMap<String, VersionReq>,
    },
    Vcpkg {
        vcpkg: HashMap<String, VersionReq>,
    },
}
