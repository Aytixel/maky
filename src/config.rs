use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use blake3::Hash;
use regex::Regex;
use serde::{Deserialize, Serialize};

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

fn default_regex() -> Vec<Regex> {
    vec![]
}

fn default_library() -> Vec<String> {
    vec![]
}

fn default_directories() -> Vec<PathBuf> {
    vec![]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_compiler")]
    #[serde(alias = "cc")]
    pub compiler: String,

    #[serde(default = "default_binaries")]
    #[serde(alias = "bin")]
    pub binaries: PathBuf,

    #[serde(default = "default_objects")]
    #[serde(alias = "obj")]
    pub objects: PathBuf,

    #[serde(default = "default_sources")]
    #[serde(alias = "src")]
    pub sources: Vec<PathBuf>,

    #[serde(default = "default_includes")]
    #[serde(alias = "inc")]
    pub includes: Vec<PathBuf>,

    #[serde(default = "default_libraries")]
    #[serde(alias = "libs")]
    pub libraries: HashMap<String, LibConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LibConfig {
    #[serde(default = "default_regex")]
    #[serde(alias = "reg")]
    #[serde(with = "serde_regex")]
    pub regex: Vec<Regex>,

    #[serde(default = "default_library")]
    #[serde(alias = "lib")]
    pub library: Vec<String>,

    #[serde(default = "default_directories")]
    #[serde(alias = "dir")]
    pub directories: Vec<PathBuf>,
}

pub fn load_hash_file(config_dir_path: &Path) -> HashMap<PathBuf, Hash> {
    let hash_file = read_to_string(config_dir_path.join("./.maky/hash")).unwrap_or_default();
    let mut hash_hashmap = HashMap::new();
    let mut hash_path = Path::new("").to_path_buf();

    for (index, line) in hash_file.lines().enumerate() {
        if index % 2 == 0 {
            hash_path = Path::new(line).to_path_buf();
        } else {
            if let Ok(hash) = Hash::from_hex(line) {
                hash_hashmap.insert(hash_path.to_path_buf(), hash);
            }
        }
    }

    hash_hashmap
}

pub fn save_hash_file(
    config_dir_path: &Path,
    hash_hashmap: &HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    let mut data = vec![];

    for hash in hash_hashmap {
        data.append(
            &mut format!("{}\n{}\n", &hash.0.to_string_lossy(), hash.1.to_hex())
                .as_bytes()
                .to_vec(),
        );
    }

    write(config_dir_path.join("./.maky/hash"), data)
}
