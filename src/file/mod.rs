use std::{
    ffi::OsStr,
    fs::{read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::{hash, Hash};
use hyperscan::{pattern, BlockDatabase, Builder, Matching, Patterns};
use once_cell::sync::Lazy;

use crate::config::ProjectConfig;

pub mod compile;
pub mod link;

static GET_IMPORTS_REGEX: Lazy<BlockDatabase> = Lazy::new(|| {
    pattern! {"//@import .*[\r\n]"; SINGLEMATCH}
        .build()
        .unwrap()
});

pub fn get_imports(code: &str) -> Vec<String> {
    let mut imports = Vec::new();

    GET_IMPORTS_REGEX
        .scan(
            code,
            &mut GET_IMPORTS_REGEX.alloc_scratch().unwrap(),
            |_, from, to, _| {
                imports.extend(
                    code[from as usize..to as usize]
                        .split(",")
                        .map(str::trim)
                        .map(str::to_string),
                );
                Matching::Continue
            },
        )
        .unwrap();

    if let Some(import) = imports.get_mut(0) {
        if let Some(last) = import.split("//@import").last() {
            *import = last.trim_start().to_string();
        }
    }

    return imports;
}

pub fn scan_dir(
    project_config: &ProjectConfig,
    dir_path: &Path,
    main_hashset: &mut AHashSet<PathBuf>,
    h_h_link: &mut AHashMap<PathBuf, AHashSet<PathBuf>>,
    h_c_link: &mut AHashMap<PathBuf, AHashSet<PathBuf>>,
    c_h_link: &mut AHashMap<PathBuf, AHashSet<PathBuf>>,
    hash_hashmap: &mut AHashMap<PathBuf, Hash>,
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
                    let includes = get_includes(&path, &project_config.includes, code);

                    if is_code_file(extension) {
                        c_h_link.insert(path.clone(), includes.clone());

                        if has_main(code) {
                            main_hashset.insert(path.clone());
                        }
                    }

                    hash_hashmap.insert(path.clone(), hash(code.as_bytes()));

                    for include in includes {
                        if is_code_file(extension) {
                            h_c_link
                                .entry(include)
                                .or_insert(AHashSet::new())
                                .insert(path.clone());
                        } else {
                            h_h_link
                                .entry(include)
                                .or_insert(AHashSet::new())
                                .insert(path.clone());
                        }
                    }
                }
            } else if path.is_dir() {
                scan_dir(
                    project_config,
                    &path,
                    main_hashset,
                    h_h_link,
                    h_c_link,
                    c_h_link,
                    hash_hashmap,
                )?;
            }
        }
    }

    return Ok(());
}

static GET_INCLUDES_REGEX: Lazy<BlockDatabase> = Lazy::new(|| {
    r#"
    /"(.*)"/
    /<(.*)>/
    "#
    .parse::<Patterns>()
    .unwrap()
    .into_iter()
    .map(|pattern| pattern.left_most())
    .collect::<Patterns>()
    .build()
    .unwrap()
});

fn get_includes(path: &Path, include_path_vec: &Vec<PathBuf>, code: &str) -> AHashSet<PathBuf> {
    let mut include_hashset = AHashSet::new();
    let parent_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();

    for line in code.lines() {
        let line = line.trim();

        if line.starts_with("#include") {
            GET_INCLUDES_REGEX
                .scan(
                    line,
                    &mut GET_INCLUDES_REGEX.alloc_scratch().unwrap(),
                    |_, from, to, _| {
                        let path = Path::new(&line[from as usize + 1..to as usize - 1]);

                        if path.is_file() {
                            include_hashset.insert(path.to_path_buf());
                            return Matching::Continue;
                        }

                        let path_with_parent = parent_path.join(path);

                        if path_with_parent.is_file() {
                            include_hashset.insert(path_with_parent.to_path_buf());
                            return Matching::Continue;
                        }

                        for include_path in include_path_vec {
                            let path = include_path.join(path);

                            if path.is_file() {
                                include_hashset.insert(path.to_path_buf());
                                return Matching::Continue;
                            }
                        }

                        Matching::Continue
                    },
                )
                .unwrap();
        }
    }

    include_hashset
}

static HAS_MAIN_REGEX: Lazy<BlockDatabase> = Lazy::new(|| {
    pattern! {r"(void|int)[ \t\n\r]*main[ \t\n\r]*\("; SINGLEMATCH}
        .build()
        .unwrap()
});

fn has_main(code: &str) -> bool {
    let mut has_found_main = false;

    HAS_MAIN_REGEX
        .scan(
            code,
            &mut HAS_MAIN_REGEX.alloc_scratch().unwrap(),
            |_, _, _, _| {
                has_found_main = true;
                Matching::Continue
            },
        )
        .unwrap();
    has_found_main
}

fn is_code_file(extension: &OsStr) -> bool {
    extension == "c"
        || extension == "cpp"
        || extension == "cxx"
        || extension == "c++"
        || extension == "cc"
}

fn is_header_file(extension: &OsStr) -> bool {
    extension == "h"
        || extension == "hpp"
        || extension == "hxx"
        || extension == "h++"
        || extension == "hh"
}
