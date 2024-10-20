use std::path::{Path, PathBuf};

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{formats::PreferOne, serde_as, OneOrMany};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageConfig {
    #[serde(serialize_with = "PackageConfig::serialize_version")]
    #[serde(deserialize_with = "PackageConfig::deserialize_version")]
    pub version: Version,

    #[serde(default = "PackageConfig::default_c_compiler")]
    #[serde(alias = "cc", rename = "c-compiler")]
    pub c_compiler: String,

    #[serde(default = "PackageConfig::default_cpp_compiler")]
    #[serde(alias = "cxx", rename = "cpp-compiler")]
    pub cpp_compiler: String,

    #[serde(alias = "std")]
    pub standard: Option<String>,

    #[serde(default = "PackageConfig::default_binaries")]
    #[serde(alias = "bin")]
    pub binaries: PathBuf,

    #[serde(default = "PackageConfig::default_objects")]
    #[serde(alias = "obj")]
    pub objects: PathBuf,

    #[serde(default = "PackageConfig::default_sources")]
    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub sources: Vec<PathBuf>,

    #[serde(default = "PackageConfig::default_includes")]
    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub includes: Vec<PathBuf>,
}

impl PackageConfig {
    fn serialize_version<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&version.to_string())
    }

    fn deserialize_version<'de, D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'de>,
    {
        let version = String::deserialize(deserializer)?;

        Ok(Version::parse(&version).expect("Malformed version in config"))
    }

    fn default_c_compiler() -> String {
        "gcc".to_string()
    }

    fn default_cpp_compiler() -> String {
        "g++".to_string()
    }

    fn default_binaries() -> PathBuf {
        Path::new("bin").to_path_buf()
    }

    fn default_objects() -> PathBuf {
        Path::new("obj").to_path_buf()
    }

    fn default_sources() -> Vec<PathBuf> {
        vec![Path::new("src").to_path_buf()]
    }

    fn default_includes() -> Vec<PathBuf> {
        vec![Path::new("include").to_path_buf()]
    }
}
