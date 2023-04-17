use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::Hash;

pub fn compile(
    objects_dir_path: &Path,
    h_h_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
    h_c_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
    hash_hashmap: &mut AHashMap<PathBuf, Hash>,
    new_hash_hashmap: &AHashMap<PathBuf, Hash>,
) -> AHashMap<PathBuf, Hash> {
    let mut files_to_compile = AHashMap::new();
    let mut new_hash_hashmap_clone = new_hash_hashmap.clone();

    for new_hash in new_hash_hashmap.clone() {
        if let Some(extension) = new_hash.0.extension() {
            if let Some(hash) = hash_hashmap.get(&new_hash.0) {
                if &new_hash.1 == hash
                    && ((extension == "c"
                        && objects_dir_path
                            .join(new_hash.1.to_hex().as_str())
                            .is_file())
                        || extension == "h")
                {
                    new_hash_hashmap_clone.remove(&new_hash.0);
                    hash_hashmap.remove(&new_hash.0);
                    continue;
                }
            }

            if extension == "c" {
                new_hash_hashmap_clone.remove(&new_hash.0);
                files_to_compile.insert(new_hash.0, new_hash.1);
            }
        }
    }

    let mut already_explored = AHashSet::new();

    for new_hash in new_hash_hashmap_clone.iter() {
        find_c_from_h(
            new_hash.0,
            h_h_link,
            h_c_link,
            new_hash_hashmap,
            &mut files_to_compile,
            &mut already_explored,
        );
    }

    for hash in hash_hashmap.iter() {
        let object_path = objects_dir_path.join(hash.1.to_hex().as_str());

        if object_path.is_file() {
            remove_file(object_path).ok();
        }
    }

    files_to_compile
}

fn find_c_from_h(
    file: &Path,
    h_h_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
    h_c_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
    new_hash_hashmap_clone: &AHashMap<PathBuf, Hash>,
    files_to_compile: &mut AHashMap<PathBuf, Hash>,
    already_explored: &mut AHashSet<PathBuf>,
) {
    if !already_explored.contains(file) {
        already_explored.insert(file.to_path_buf());

        if let Some(files) = h_c_link.get(file) {
            for file in files.iter() {
                files_to_compile.insert(file.to_path_buf(), new_hash_hashmap_clone[file]);
            }
        }
        if let Some(files) = h_h_link.get(file) {
            for file in files.clone() {
                find_c_from_h(
                    &file,
                    h_h_link,
                    h_c_link,
                    new_hash_hashmap_clone,
                    files_to_compile,
                    already_explored,
                );
            }
        }
    }
}
