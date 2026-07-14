use std::path::PathBuf;

use parse_git_url::GitUrl;
use semver::VersionReq;
use serde::Deserialize;
use serde_with::{OneOrMany, formats::PreferOne, serde_as};
use tuple_vec_map;

use crate::{
    config::{VecOrValue, replace_path_templates, require::Require},
    git::GitRepositoryInfo,
    helpers,
};

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct Dependency {
    #[serde(default)]
    pub(in crate::config) require: Require,

    #[serde(default)]
    pub defines: Vec<String>,
    #[serde(default)]
    pub cflags: Vec<String>,
    #[serde(default)]
    pub lflags: Vec<String>,

    #[serde(flatten)]
    pub dependency: TypedDependency,
}

impl Dependency {
    pub(in crate::config) fn apply_path_templates(mut self) -> Self {
        match &mut self.dependency {
            TypedDependency::Pkg { paths: path, .. } => {
                *path = path.values().map(replace_path_templates).collect();
            }
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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum MakyPathDependency {
    Git {
        #[serde(rename = "git")]
        url: String,
        rev: Option<String>,
        path: Option<String>,
    },
    Local {
        path: String,
    },
}

impl MakyPathDependency {
    pub fn path(
        &self,
        project_paths: &helpers::ProjectPaths,
        name: &str,
    ) -> anyhow::Result<(PathBuf, Option<GitRepositoryInfo>)> {
        Ok(match self {
            MakyPathDependency::Git { url, rev, path } => {
                let project_path = project_paths.maky_dependencies_path.join(name);

                (
                    if let Some(path) = path {
                        project_path.join(path)
                    } else {
                        project_path.clone()
                    },
                    Some(GitRepositoryInfo {
                        url: GitUrl::parse(&url)?,
                        rev: rev.clone(),
                        path: project_path,
                    }),
                )
            }
            MakyPathDependency::Local { path } => (project_paths.project_path.join(path), None),
        })
    }
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TypedDependency {
    Pkg {
        #[serde(alias = "path", default)]
        paths: VecOrValue<String>,

        #[serde(with = "tuple_vec_map")]
        pkg: Vec<(String, VersionReq)>,
    },
    Maky {
        #[serde(default)]
        version: Option<VersionReq>,

        #[serde(flatten)]
        path: MakyPathDependency,

        #[serde(alias = "lib", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        libraries: Vec<String>,
    },
    Local {
        #[serde(alias = "inc", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        includes: Vec<String>,

        #[serde(alias = "dir", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        directories: Vec<String>,

        #[serde(alias = "lib", default)]
        #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
        libraries: Vec<String>,
    },
}
