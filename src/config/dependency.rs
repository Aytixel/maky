use std::collections::HashMap;

use semver::VersionReq;
use serde::Deserialize;
use serde_with::{formats::PreferOne, serde_as, OneOrMany};

use crate::config::{replace_path_templates, require::Require};

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct Dependency {
    #[serde(default)]
    pub(in crate::config) require: Require,

    #[serde(flatten)]
    pub dependency: TypedDependency,
}

impl Dependency {
    pub(in crate::config) fn apply_path_templates(mut self) -> Self {
        match &mut self.dependency {
            TypedDependency::Local {
                directories,
                includes,
                ..
            } => {
                *directories = directories
                    .into_iter()
                    .map(replace_path_templates)
                    .collect();
                *includes = includes.into_iter().map(replace_path_templates).collect();
            }
            _ => {}
        }

        self
    }
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TypedDependency {
    MakyLocal {
        #[serde(default)]
        version: Option<VersionReq>,

        path: String,

        #[serde(alias = "lib", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        library: Vec<String>,
    },
    MakyGit {
        #[serde(default)]
        version: Option<VersionReq>,

        git: String,

        rev: Option<String>,

        #[serde(alias = "lib", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        library: Vec<String>,
    },
    Local {
        #[serde(alias = "dir", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        directories: Vec<String>,

        #[serde(alias = "inc", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        includes: Vec<String>,

        #[serde(alias = "lib", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        library: Vec<String>,
    },
    PkgSystem {
        pkg: HashMap<String, VersionReq>,
    },
    PkgLocal {
        pkg: String,
    },
}
