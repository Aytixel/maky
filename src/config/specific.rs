use std::path::PathBuf;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use serde_with::{formats::PreferOne, serde_as, OneOrMany};

use super::lib::LibConfig;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SpecificConfig {
    #[serde(alias = "cc", rename = "c-compiler")]
    pub c_compiler: Option<String>,

    #[serde(alias = "cxx", rename = "cpp-compiler")]
    pub cpp_compiler: Option<String>,

    #[serde(alias = "bin")]
    pub binaries: Option<PathBuf>,

    #[serde(alias = "obj")]
    pub objects: Option<PathBuf>,

    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "Option<OneOrMany<_, PreferOne>>")]
    pub sources: Option<Vec<PathBuf>>,

    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "Option<OneOrMany<_, PreferOne>>")]
    pub includes: Option<Vec<PathBuf>>,

    #[serde(alias = "libs")]
    pub libraries: Option<HashMap<String, LibConfig>>,
}
