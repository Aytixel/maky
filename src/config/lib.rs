use std::path::PathBuf;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use serde_with::{formats::PreferOne, serde_as, OneOrMany};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibConfig {
    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "lib")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub library: Vec<String>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "dir")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub directories: Vec<PathBuf>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub includes: Vec<PathBuf>,

    #[serde(default = "LibConfig::default_hashmap")]
    #[serde(alias = "pkg")]
    pub pkg_config: HashMap<String, String>,
}

impl LibConfig {
    fn default_vec<T>() -> Vec<T> {
        Vec::new()
    }

    fn default_hashmap<T>() -> HashMap<String, T> {
        HashMap::new()
    }
}
