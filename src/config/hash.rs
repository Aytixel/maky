use std::{
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use blake3::Hash;
use hashbrown::HashMap;

fn get_hash_path(project_path: &Path, release: bool) -> PathBuf {
    project_path.join(format!(
        ".maky/{}_hash",
        release.then_some("release").unwrap_or("debug")
    ))
}

pub trait LoadHash {
    fn load(path: &Path, release: bool) -> io::Result<Self>
    where
        Self: Sized;
}

impl LoadHash for HashMap<PathBuf, Hash> {
    fn load(project_path: &Path, release: bool) -> io::Result<Self> {
        let hash_file = read_to_string(get_hash_path(project_path, release))?;
        let mut hash_hashmap = HashMap::new();
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

pub trait SaveHash {
    fn save(&self, path: &Path, release: bool) -> io::Result<()>;
}

impl SaveHash for HashMap<PathBuf, Hash> {
    fn save(&self, project_path: &Path, release: bool) -> io::Result<()> {
        let mut data = Vec::new();

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

        write(get_hash_path(project_path, release), data)
    }
}
