use std::{
    fs::remove_file,
    path::{Path, PathBuf},
};

use blake3::Hash;
use hashbrown::{HashMap, HashSet};

use super::{is_code_file, is_header_file};

pub fn compile(
    objects_dir_path: &Path,
    h_h_link: &HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
    new_hash_hashmap: &HashMap<PathBuf, Hash>,
) -> HashMap<PathBuf, Hash> {
    let mut files_to_compile = HashMap::new();
    let mut new_hash_hashmap_clone = new_hash_hashmap.clone();

    for new_hash in new_hash_hashmap.iter() {
        if let Some(extension) = new_hash.0.extension() {
            if let Some(hash) = hash_hashmap.get(new_hash.0) {
                if new_hash.1 == hash
                    && ((is_code_file(extension)
                        && objects_dir_path
                            .join(new_hash.1.to_hex().as_str())
                            .is_file())
                        || is_header_file(extension))
                {
                    new_hash_hashmap_clone.remove(new_hash.0);
                    hash_hashmap.remove(new_hash.0);
                    continue;
                }
            }

            if is_code_file(extension) {
                new_hash_hashmap_clone.remove(new_hash.0);
                files_to_compile.insert(new_hash.0.clone(), new_hash.1.clone());
            }
        }
    }

    let mut already_explored = HashSet::new();

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
    h_h_link: &HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &HashMap<PathBuf, HashSet<PathBuf>>,
    new_hash_hashmap: &HashMap<PathBuf, Hash>,
    files_to_compile: &mut HashMap<PathBuf, Hash>,
    already_explored: &mut HashSet<PathBuf>,
) {
    if !already_explored.contains(file) {
        already_explored.insert(file.to_path_buf());

        if let Some(files) = h_c_link.get(file) {
            for file in files.iter() {
                files_to_compile.insert(file.to_path_buf(), new_hash_hashmap[file]);
            }
        }
        if let Some(files) = h_h_link.get(file) {
            for file in files.iter() {
                find_c_from_h(
                    file,
                    h_h_link,
                    h_c_link,
                    new_hash_hashmap,
                    files_to_compile,
                    already_explored,
                );
            }
        }
    }
}
