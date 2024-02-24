use std::{
    ffi::OsStr,
    fs::{read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::{hash, Hash};
use hyperscan::{BlockDatabase, Builder, Matching, Patterns};
use once_cell::sync::Lazy;
use tree_sitter::{Language, Parser};

use crate::config::ProjectConfig;

pub mod compile;
pub mod link;

pub fn get_imports(code: &str, extension: &OsStr) -> Vec<String> {
    if let Some(tree) = {
        let mut parser = Parser::new();

        parser
            .set_language(get_language(extension).expect("Unknown file extension"))
            .expect("Error loading parser grammar");
        parser.parse(code, None)
    } {
        for i in 0..tree.root_node().child_count() {
            let node = tree.root_node().child(i).unwrap();
            let code = &code[node.byte_range()];

            if node.kind() == "comment" && code.starts_with("//@import ") {
                return code[10..]
                    .split(",")
                    .map(str::trim)
                    .map(str::to_string)
                    .collect::<Vec<String>>();
            }
        }
    }

    return Vec::new();
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

                        if has_main(code, extension) {
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

fn has_main(code: &str, extension: &OsStr) -> bool {
    if let Some(tree) = {
        let mut parser = Parser::new();

        parser
            .set_language(get_language(extension).expect("Unknown file extension"))
            .expect("Error loading parser grammar");
        parser.parse(code, None)
    } {
        for i in 0..tree.root_node().child_count() {
            if let Some(node) = tree.root_node().child(i).unwrap().child(1) {
                if node.kind() == "function_declarator"
                    && node.child_count() > 0
                    && &code[node.child(0).unwrap().byte_range()] == "main"
                {
                    return true;
                }
            }
        }
    }

    false
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

fn get_language(extension: &OsStr) -> Option<Language> {
    match extension.to_str().unwrap_or_default() {
        "c" | "h" => Some(tree_sitter_c::language()),
        "cc" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" => {
            Some(tree_sitter_cpp::language())
        }
        _ => None,
    }
}
