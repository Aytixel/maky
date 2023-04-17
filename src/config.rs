use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use ahash::AHashMap;
use blake3::Hash;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub trait LoadConfig {
    fn load(path: &Path) -> io::Result<Self>
    where
        Self: Sized;
}

pub trait SaveConfig {
    fn save(&self, path: &Path) -> io::Result<()>;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default = "Config::default_bool")]
    pub release: bool,
}

impl Config {
    fn default_bool() -> bool {
        false
    }
}

impl LoadConfig for Config {
    fn load(project_path: &Path) -> io::Result<Self> {
        toml::from_str(&read_to_string(project_path.join(".maky/config.toml"))?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
    }
}

impl SaveConfig for Config {
    fn save(&self, project_path: &Path) -> io::Result<()> {
        write(
            project_path.join("./.maky/config.toml"),
            toml::to_string_pretty(self)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    #[serde(default = "ProjectConfig::default_compiler")]
    #[serde(alias = "cc")]
    pub compiler: String,

    #[serde(default = "ProjectConfig::default_binaries")]
    #[serde(alias = "bin")]
    pub binaries: PathBuf,

    #[serde(default = "ProjectConfig::default_objects")]
    #[serde(alias = "obj")]
    pub objects: PathBuf,

    #[serde(default = "ProjectConfig::default_sources")]
    #[serde(alias = "src")]
    pub sources: Vec<PathBuf>,

    #[serde(default = "ProjectConfig::default_includes")]
    #[serde(alias = "inc")]
    pub includes: Vec<PathBuf>,

    #[serde(default = "ProjectConfig::default_libraries")]
    #[serde(alias = "libs")]
    pub libraries: HashMap<String, LibConfig>,
}

impl ProjectConfig {
    fn default_compiler() -> String {
        "gcc".to_string()
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
        vec![]
    }

    fn default_libraries() -> HashMap<String, LibConfig> {
        HashMap::new()
    }
}

impl LoadConfig for ProjectConfig {
    fn load(file_path: &Path) -> io::Result<Self> {
        toml::from_str(&read_to_string(file_path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibConfig {
    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "reg")]
    #[serde(with = "serde_regex")]
    pub regex: Vec<Regex>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "lib")]
    pub library: Vec<String>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "dir")]
    pub directories: Vec<PathBuf>,
}

impl LibConfig {
    fn default_vec<T>() -> Vec<T> {
        vec![]
    }
}

impl LoadConfig for AHashMap<PathBuf, Hash> {
    fn load(project_path: &Path) -> io::Result<Self> {
        let hash_file = read_to_string(project_path.join(".maky/hash"))?;
        let mut hash_hashmap = AHashMap::new();
        let mut hash_path = Path::new("");

        for (index, line) in hash_file.lines().enumerate() {
            if index % 2 == 0 {
                hash_path = Path::new(line);
            } else {
                if let Ok(hash) = Hash::from_hex(line) {
                    hash_hashmap.insert(project_path.join(hash_path), hash);
                }
            }
        }

        Ok(hash_hashmap)
    }
}

impl SaveConfig for AHashMap<PathBuf, Hash> {
    fn save(&self, project_path: &Path) -> io::Result<()> {
        let mut data = vec![];

        for hash in self {
            data.append(
                &mut format!(
                    "{}\n{}\n",
                    &hash
                        .0
                        .strip_prefix(project_path)
                        .unwrap_or(hash.0)
                        .to_string_lossy(),
                    hash.1.to_hex()
                )
                .as_bytes()
                .to_vec(),
            );
        }

        write(project_path.join("./.maky/hash"), data)
    }
}
