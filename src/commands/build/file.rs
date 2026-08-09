use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use async_walkdir::WalkDir;
use blake3::Hash;
use futures_lite::StreamExt;
use tokio::{
    fs::{read_to_string, write},
    task::JoinSet,
};

use crate::{
    config::{self, Target},
    file::{Language, Node, get_ast, is_code_file, is_header_file},
    helpers::{self, TryStripPrefixPath},
};

fn is_valid_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let Some(extension) = path.extension() else {
        return false;
    };

    is_code_file(extension) || is_header_file(extension)
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub code: String,
    pub hash: Hash,
    pub ast: Node,
    pub language: Language,
    pub dependencies: HashSet<PathBuf>,
}

pub async fn scan_source_files(
    targets: &[Target],
    project_paths: &helpers::ProjectPaths,
    package_config: &config::Package,
    include_paths: &[PathBuf],
) -> anyhow::Result<HashMap<PathBuf, SourceFile>> {
    let mut source_files = HashSet::new();

    for source in &package_config.sources {
        let mut entries = WalkDir::new(project_paths.project_path.join(source));

        while let Some(entry) = entries.try_next().await? {
            let path = entry.path();

            if is_valid_file(&path) {
                source_files.insert(path);
            }
        }
    }

    source_files.extend(
        targets
            .iter()
            .map(|target| project_paths.project_path.join(&target.path)),
    );

    let mut visited_files = HashSet::new();
    let mut next_files = source_files;
    let mut source_files = HashMap::new();

    while !next_files.is_empty() {
        let new_source_files = visit_source_files(
            &project_paths.project_path,
            &project_paths.maky_ast_path,
            include_paths,
            next_files.clone(),
            &mut visited_files,
        )
        .await?;

        next_files = new_source_files
            .values()
            .flat_map(|source_file| &source_file.dependencies)
            .map(|path| project_paths.project_path.join(path))
            .collect();

        source_files.extend(new_source_files);
    }

    Ok(source_files)
}

async fn visit_source_files(
    project_path: &Path,
    maky_ast_path: &Path,
    include_paths: &[PathBuf],
    files: HashSet<PathBuf>,
    visited_files: &mut HashSet<PathBuf>,
) -> anyhow::Result<HashMap<PathBuf, SourceFile>> {
    let files = JoinSet::from_iter(files.into_iter().filter_map(|file| {
        if visited_files.contains(&file) {
            return None;
        }

        visited_files.insert(file.clone());

        let maky_ast_path = maky_ast_path.to_path_buf();

        Some(async {
            let mut parser = tree_sitter::Parser::new();
            let (code, hash, ast, language) = get_ast(&mut parser, maky_ast_path, &file)
                .await
                .map_err(|error| anyhow!("`{}` {error}", file.display()))?;

            Ok((file, code, hash, ast, language))
        })
    }));

    files
        .join_all()
        .await
        .into_iter()
        .map(|file_ast| {
            file_ast.and_then(|(path, code, hash, ast, language)| {
                let includes = get_includes(&path, project_path, include_paths, &code)?;
                let path = path.try_strip_prefix(project_path);

                Ok((
                    path,
                    SourceFile {
                        code,
                        hash,
                        ast,
                        language,
                        dependencies: includes,
                    },
                ))
            })
        })
        .collect()
}

const INCLUDE_PATTERN: &str = "#include";

fn get_includes(
    path: &Path,
    project_path: &Path,
    include_paths: &[PathBuf],
    code: &str,
) -> anyhow::Result<HashSet<PathBuf>> {
    let mut include_hashset = HashSet::new();
    let parent_path = path
        .parent()
        .ok_or(anyhow!(
            "can't find parent directory of `{}`",
            path.display()
        ))?
        .to_path_buf();

    'main: for (index, _) in code.match_indices(INCLUDE_PATTERN) {
        let index = index + INCLUDE_PATTERN.len();

        if let Some((code, _)) = code[index..].trim_start()[1..].split_once(['"', '>']) {
            let path = Path::new(code);
            let path_with_parent = parent_path.join(path);

            // try current directory
            if path_with_parent.is_file() {
                include_hashset.insert(path_with_parent.try_strip_prefix(project_path));
                continue 'main;
            }

            // try include paths
            for include_path in include_paths {
                let path_with_parent = include_path.join(path);

                if path_with_parent.is_file() {
                    include_hashset.insert(path_with_parent.try_strip_prefix(project_path));
                    continue 'main;
                }
            }
        }
    }

    Ok(include_hashset)
}

pub async fn check_updated_source_files(
    project_paths: &helpers::ProjectPaths,
    source_files: &HashMap<PathBuf, SourceFile>,
) -> anyhow::Result<HashSet<PathBuf>> {
    let mut old_source_files_hash = HashMap::new();

    if let Ok(content) = read_to_string(&project_paths.maky_hash_path).await {
        for line in content.lines() {
            if let Some((Ok(hash), path)) = line
                .split_once(" ")
                .map(|(hash, path)| (Hash::from_hex(hash), path))
            {
                old_source_files_hash.insert(Path::new(path).to_path_buf(), hash);
            }
        }
    }

    let mut updated_source_files = HashSet::new();
    let mut new_source_files_hash = Vec::new();

    for (path, source_file) in source_files {
        let path = path.try_strip_prefix(&project_paths.project_path);

        new_source_files_hash
            .extend(format!("{} {}\n", source_file.hash, path.to_string_lossy()).as_bytes());

        if old_source_files_hash
            .get(&path)
            .map(|old_hash| old_hash != &source_file.hash)
            .unwrap_or(true)
        {
            updated_source_files.insert(path.to_path_buf());
        }
    }

    write(&project_paths.maky_hash_path, new_source_files_hash)
        .await
        .map_err(|_| anyhow!("can't write maky hash file"))?;

    Ok(updated_source_files)
}

pub fn get_source_files_reverse_dependencies(
    source_files: &HashMap<PathBuf, SourceFile>,
) -> HashMap<PathBuf, SourceFile> {
    let mut source_files_reverse_dependencies = HashMap::new();

    for (path, source_file) in source_files {
        for include in &source_file.dependencies {
            source_files_reverse_dependencies
                .entry(include.clone())
                .or_insert_with(|| {
                    let mut included_source_file = source_files[include].clone();

                    included_source_file.dependencies.clear();
                    included_source_file
                })
                .dependencies
                .insert(path.clone());
        }
    }

    source_files_reverse_dependencies
}

pub fn add_uncompiled_source_files(
    objects_path: &Path,
    source_files: &HashMap<PathBuf, SourceFile>,
    mut updated_source_files: HashSet<PathBuf>,
) -> HashSet<PathBuf> {
    for (path, source_file) in source_files {
        let Some(extension) = path.extension() else {
            continue;
        };

        if is_code_file(extension) {
            if !objects_path.join(source_file.hash.to_string()).is_file() {
                updated_source_files.insert(path.clone());
            }
        }
    }

    updated_source_files
}

pub fn add_source_files_dependencies(
    source_files: &HashMap<PathBuf, SourceFile>,
    mut updated_source_files: HashSet<PathBuf>,
) -> anyhow::Result<HashSet<PathBuf>> {
    let mut files = updated_source_files.clone();
    let mut visited_files = HashSet::new();

    while !files.is_empty() {
        let new_files_dependencies =
            visit_source_files_dependencies(source_files, files.clone(), &mut visited_files)?;

        files = new_files_dependencies.clone();
        updated_source_files.extend(new_files_dependencies);
    }

    Ok(updated_source_files)
}

fn visit_source_files_dependencies(
    source_files: &HashMap<PathBuf, SourceFile>,
    files: HashSet<PathBuf>,
    visited_files: &mut HashSet<PathBuf>,
) -> anyhow::Result<HashSet<PathBuf>> {
    let mut files_dependencies = HashSet::new();

    for file in &files {
        if visited_files.contains(file) {
            continue;
        }

        visited_files.insert(file.clone());

        if let Some(source_file) = source_files.get(file) {
            files_dependencies.extend(source_file.dependencies.clone());
        }
    }

    Ok(files_dependencies)
}

pub fn filter_source_files(mut source_files: HashSet<PathBuf>) -> HashSet<PathBuf> {
    source_files.retain(|file| file.extension().map(is_code_file).unwrap_or_default());
    source_files
}

pub fn filter_header_files(mut header_files: HashSet<PathBuf>) -> HashSet<PathBuf> {
    header_files.retain(|file| file.extension().map(is_header_file).unwrap_or_default());
    header_files
}

pub async fn get_targets_source_files<'a>(
    targets: &'a [Target],
    source_files: &HashMap<PathBuf, SourceFile>,
    source_files_reverse_dependencies: &HashMap<PathBuf, SourceFile>,
) -> anyhow::Result<HashMap<&'a Target, HashSet<PathBuf>>> {
    let mut targets_source_files = HashMap::new();

    for target in targets {
        let target_path = PathBuf::from(target.path.clone());
        let header_files = filter_header_files(add_source_files_dependencies(
            source_files,
            HashSet::from([target_path.clone()]),
        )?);
        let code_files = filter_source_files(
            header_files
                .iter()
                .filter_map(|file| {
                    source_files_reverse_dependencies
                        .get(file)
                        .map(|source_file| &source_file.dependencies)
                })
                .flatten()
                .cloned()
                .collect::<HashSet<_>>(),
        );
        let header_prototype_signatures: HashSet<String> = header_files
            .into_iter()
            .filter_map(|header_file| source_files.get(&header_file))
            .flat_map(|source_file| {
                get_prototype_signatures(&source_file.ast, &source_file.code, Vec::new())
            })
            .collect();
        let mut target_source_files: HashSet<PathBuf> = code_files
            .into_iter()
            .filter(|code_file| {
                source_files.get(code_file).map_or(false, |source_file| {
                    get_prototype_signatures(&source_file.ast, &source_file.code, Vec::new())
                        .iter()
                        .any(|prototype_signature| {
                            header_prototype_signatures.contains(prototype_signature)
                        })
                })
            })
            .collect();

        target_source_files.insert(target_path);
        targets_source_files.insert(target, target_source_files);
    }

    Ok(targets_source_files)
}

fn get_prototype_signatures(node: &Node, code: &str, mut class_path: Vec<String>) -> Vec<String> {
    if node.name == "class_specifier"
        && let Some(node) = node.children.get(1)
        && node.name == "identifier"
    {
        class_path.push(code[node.start..node.end].to_string());
    }

    if node.name == "function_definition"
        || node.name == "declaration"
        || node.name == "field_declaration"
    {
        let mut node = remove_params_identifer(node.clone());

        node.children = node
            .children
            .into_iter()
            .filter(|node| {
                node.name != "storage_class_specifier"
                    && node.name != "compound_statement"
                    && node.name != ";"
            })
            .collect();

        let mut declarations_node: Vec<Node> = node
            .children
            .extract_if(.., |node| node.name == "declaration")
            .map(|mut node| {
                node.children.pop_if(|node| node.name == ";");
                node
            })
            .collect();

        // combine argument and function declaration when separate
        if !declarations_node.is_empty()
            && let Some(node) = node.children.get_mut(1)
            && node.name == "function_declarator"
            && let Some(node) = node.children.get_mut(1)
            && node.name == "parameter_list"
        {
            declarations_node.reverse();

            node.children = node
                .children
                .clone()
                .into_iter()
                .map(|node| {
                    if node.name == "identifier" {
                        declarations_node.pop().unwrap_or(node)
                    } else {
                        node
                    }
                })
                .collect();
        }

        if let Some(node) = node.children.last_mut() {
            node.children = class_path
                .into_iter()
                .flat_map(|class| {
                    [
                        Node {
                            name: class,
                            start: 0,
                            end: 0,
                            children: Vec::new(),
                        },
                        Node {
                            name: "::".to_string(),
                            start: 0,
                            end: 0,
                            children: Vec::new(),
                        },
                    ]
                })
                .chain(node.children.clone())
                .collect();
        }

        return vec![generate_prototype_signature(&node, code)];
    }

    node.children
        .iter()
        .flat_map(|node| get_prototype_signatures(node, code, class_path.clone()))
        .collect()
}

fn generate_prototype_signature(node: &Node, code: &str) -> String {
    if node.start == 0 && node.end == 0 {
        return node.name.clone();
    }

    if node.children.is_empty() {
        return code[node.start..node.end].to_string();
    }

    node.children
        .iter()
        .map(|node| generate_prototype_signature(node, code))
        .collect::<Vec<String>>()
        .join(" ")
}

fn remove_params_identifer(mut node: Node) -> Node {
    node.children = node
        .children
        .into_iter()
        .map(|mut node| {
            if (node.name == "parameter_declaration" || node.name == "declaration")
                && let Some(identifer_node) = node.children.get(1)
            {
                if identifer_node.name == "identifier" {
                    node.children.remove(1);
                } else {
                    node = remove_identifier(node);
                }
            }

            remove_params_identifer(node)
        })
        .collect();
    node
}

fn remove_identifier(mut node: Node) -> Node {
    node.children = node
        .children
        .into_iter()
        .filter(|node| node.name != "identifier")
        .map(remove_identifier)
        .collect();
    node
}
