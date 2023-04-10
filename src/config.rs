use std::{
    collections::HashMap,
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use blake3::Hash;

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
