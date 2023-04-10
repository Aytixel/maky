use std::{
    collections::{HashMap, HashSet},
    fs::remove_file,
    io,
    path::{Path, PathBuf},
    process::Command,
};

use blake3::Hash;

use crate::{Args, Config};

pub fn compile(
    args: &Args,
    config: &Config,
    objects_dir_path: &Path,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
    new_hash_hashmap: &mut HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    let mut file_to_compile = HashMap::new();
    let new_hash_hashmap_clone = new_hash_hashmap.clone();

    for new_hash in new_hash_hashmap_clone.clone() {
        let extension = new_hash.0.extension().unwrap_or_default();

        if let Some(hash) = hash_hashmap.get(&new_hash.0) {
            if &new_hash.1 == hash
                && ((extension == "c"
                    && objects_dir_path
                        .join(new_hash.1.to_hex().as_str())
                        .is_file())
                    || extension == "h")
            {
                new_hash_hashmap.remove(&new_hash.0);
                hash_hashmap.remove(&new_hash.0);
                continue;
            }
        }

        if extension == "c" {
            new_hash_hashmap.remove(&new_hash.0);
            file_to_compile.insert(new_hash.0, new_hash.1);
        }
    }

    let mut already_explored = HashSet::new();

    for new_hash in new_hash_hashmap.iter() {
        find_c_from_h(
            new_hash.0,
            h_h_link,
            h_c_link,
            &new_hash_hashmap_clone,
            &mut file_to_compile,
            &mut already_explored,
        );
    }

    for hash in hash_hashmap.iter() {
        let object_path = objects_dir_path.join(hash.1.to_hex().as_str());

        if object_path.is_file() {
            remove_file(object_path)?;
        }
    }

    drop(hash_hashmap);
    drop(new_hash_hashmap);
    drop(new_hash_hashmap_clone);
    drop(already_explored);

    print!("{} file", file_to_compile.len());

    if file_to_compile.len() > 1 {
        print!("s");
    }

    print!(" to compile");

    if file_to_compile.len() > 0 {
        println!(" :");
    } else {
        println!("");
    }

    let mut commands = vec![];

    for file in file_to_compile {
        println!("  - {}", &file.0.to_string_lossy());

        let mut command = Command::new("gcc");

        if !args.release {
            command.arg("-g").arg("-Wall");
        }

        for include in config.includes.iter() {
            command.arg("-I").arg(include);
        }

        commands.push(
            command
                .arg("-c")
                .arg(file.0)
                .arg("-o")
                .arg(objects_dir_path.join(file.1.to_hex().as_str()))
                .spawn()
                .unwrap(),
        );
    }

    for command in commands.iter_mut() {
        command.wait().unwrap();
    }

    Ok(())
}

fn find_c_from_h(
    file: &Path,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    new_hash_hashmap_clone: &HashMap<PathBuf, Hash>,
    file_to_compile: &mut HashMap<PathBuf, Hash>,
    already_explored: &mut HashSet<PathBuf>,
) {
    if !already_explored.contains(file) {
        already_explored.insert(file.to_path_buf());

        if let Some(files) = h_c_link.get(file) {
            for file in files.iter() {
                file_to_compile.insert(file.to_path_buf(), new_hash_hashmap_clone[file]);
            }
        }
        if let Some(files) = h_h_link.get(file) {
            for file in files.clone() {
                find_c_from_h(
                    &file,
                    h_h_link,
                    h_c_link,
                    new_hash_hashmap_clone,
                    file_to_compile,
                    already_explored,
                );
            }
        }
    }
}
