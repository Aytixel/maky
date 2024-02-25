use std::{
    ffi::OsStr,
    fs::{read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::{hash, Hash};
use tree_sitter::{Language, Parser, Query, QueryCursor};

use crate::config::ProjectConfig;

pub mod compile;
pub mod link;

const IMPORT_LENGTH: usize = 10;

pub fn get_imports(code: &str) -> Vec<String> {
    if let Some(index) = code.find("//@import ") {
        let index = index + IMPORT_LENGTH;

        return code[index..index + code[index..].find("\n").expect("No end of line found")]
            .trim()
            .split(",")
            .map(str::trim)
            .map(str::to_string)
            .collect::<Vec<String>>();
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

const INCLUDE_LENGTH: usize = 8;

fn get_includes(path: &Path, include_path_vec: &Vec<PathBuf>, code: &str) -> AHashSet<PathBuf> {
    let mut include_hashset = AHashSet::new();
    let parent_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();

    'main: for (index, _) in code.match_indices("#include") {
        let index = index + INCLUDE_LENGTH;
        let code =
            code[index..index + code[index..].find("\n").expect("No end of line found")].trim();

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
                let path = include_path.join(path);

                if path.is_file() {
                    include_hashset.insert(path.to_path_buf());
                    continue 'main;
                }
            }
        }
    }

    include_hashset
}

fn has_main(code: &str, extension: &OsStr) -> bool {
    let language = get_language(extension).expect("Unknown file extension");
    let tree = {
        let mut parser = Parser::new();

        parser
            .set_language(language)
            .expect("Error loading parser grammar");
        parser.parse(code, None).expect("Error parsing file")
    };
    let query = Query::new(
        language,
        r#"
        (function_declarator
            declarator: (identifier) @name
        )
        "#,
    )
    .expect("Error building parser query");
    let mut query_cursor = QueryCursor::new();
    let query_matches = query_cursor.matches(&query, tree.root_node(), code.as_bytes());

    for query_match in query_matches {
        if &code[query_match.captures[0].node.byte_range()] == "main" {
            return true;
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
