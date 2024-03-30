use std::{
    ffi::OsStr,
    fs::{read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
};

use aho_corasick::AhoCorasick;
use blake3::{hash, Hash};
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;

use crate::config::ProjectConfig;

pub mod compile;
pub mod link;

lazy_static! {
    static ref PATTERN_MATCHER: AhoCorasick =
        AhoCorasick::new(&["//@main", "//@lib", "//@import "])
            .expect("Failed to initialize AhoCorasick pattern matcher");
}

pub fn scan_dir(
    project_path: &Path,
    project_config: &ProjectConfig,
    dir_path: &Path,
    main_hashmap: &mut HashMap<PathBuf, Option<String>>,
    lib_hashmap: &mut HashMap<PathBuf, Option<String>>,
    import_hashmap: &mut HashMap<PathBuf, Vec<String>>,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    for entry in read_dir(dir_path)? {
        if let Ok(entry) = entry {
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().unwrap_or_default();

                if is_code_file(extension) || is_header_file(extension) {
                    let code = &read_to_string(&path).map_err(|error| {
                        io::Error::new(
                            error.kind(),
                            format!("{} : {}", path.to_string_lossy(), error),
                        )
                    })?;
                    let includes =
                        get_includes(&path, project_path, &project_config.includes, code);

                    if is_code_file(extension) {
                        c_h_link.insert(path.clone(), includes.clone());

                        for match_ in PATTERN_MATCHER.find_iter(code) {
                            let line_option = code[match_.end()..]
                                .lines()
                                .next()
                                .map(|line| line.trim().to_string())
                                .filter(|line| !line.is_empty());

                            match match_.pattern().as_usize() {
                                0 => {
                                    main_hashmap.insert(path.clone(), line_option);
                                }
                                1 => {
                                    lib_hashmap.insert(path.clone(), line_option);
                                }
                                2 => {
                                    if let Some(line) = line_option {
                                        import_hashmap.insert(
                                            path.clone(),
                                            line.split(",")
                                                .map(str::trim)
                                                .map(str::to_string)
                                                .collect::<Vec<String>>(),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    hash_hashmap.insert(path.clone(), hash(code.as_bytes()));

                    for include in includes {
                        if is_code_file(extension) {
                            h_c_link
                                .entry(include)
                                .or_insert(HashSet::new())
                                .insert(path.clone());
                        } else {
                            h_h_link
                                .entry(include)
                                .or_insert(HashSet::new())
                                .insert(path.clone());
                        }
                    }
                }
            } else if path.is_dir() {
                scan_dir(
                    project_path,
                    project_config,
                    &path,
                    main_hashmap,
                    lib_hashmap,
                    import_hashmap,
                    h_h_link,
                    h_c_link,
                    c_h_link,
                    hash_hashmap,
                )?;
            }
        }
    }

    Ok(())
}

pub fn scan_dir_dependency(dir_path: &Path) -> io::Result<Vec<PathBuf>> {
    let mut h_files = Vec::new();

    for entry in read_dir(dir_path)? {
        if let Ok(entry) = entry {
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().unwrap_or_default();

                if is_header_file(extension) {
                    h_files.push(path);
                }
            } else if path.is_dir() {
                h_files.extend(scan_dir_dependency(&path)?);
            }
        }
    }

    Ok(h_files)
}

const INCLUDE_PATTERN: &str = "#include";

fn get_includes(
    path: &Path,
    project_path: &Path,
    include_path_vec: &Vec<PathBuf>,
    code: &str,
) -> HashSet<PathBuf> {
    let mut include_hashset = HashSet::new();
    let parent_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();

    'main: for (index, _) in code.match_indices(INCLUDE_PATTERN) {
        let index = index + INCLUDE_PATTERN.len();
        let code = code[index..]
            .lines()
            .next()
            .expect("Unexpected end of file")
            .trim();

        if code.len() > 2 {
            let path = Path::new(&code[1..code.len() - 1]);

            if path.is_file() {
                include_hashset.insert(path.to_path_buf());
                continue 'main;
            }

            let path_with_parent = parent_path.join(path);

            if path_with_parent.is_file() {
                include_hashset.insert(path_with_parent.to_path_buf());
                continue 'main;
            }

            for include_path in include_path_vec {
                let path = project_path.join(include_path).join(path);

                if path.is_file() {
                    include_hashset.insert(path.to_path_buf());
                    continue 'main;
                }
            }
        }
    }

    include_hashset
}

fn is_code_file(extension: &OsStr) -> bool {
    extension == "c"
        || extension == "cc"
        || extension == "cpp"
        || extension == "cxx"
        || extension == "c++"
}

fn is_header_file(extension: &OsStr) -> bool {
    extension == "h"
        || extension == "hh"
        || extension == "hpp"
        || extension == "hxx"
        || extension == "h++"
}

pub enum Language {
    C,
    Cpp,
}

pub fn get_language(extension: &OsStr) -> Language {
    match extension.to_string_lossy().to_string().as_str() {
        "c" | "h" => Language::C,
        "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" => Language::Cpp,
        _ => Language::C,
    }
}
