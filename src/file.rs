use std::{ffi::OsStr, path::Path};

use blake3::Hash;
use nanoserde::{DeJson, SerJson};
use tokio::fs::read_to_string;

#[derive(Debug, Clone, Copy)]
pub enum Language {
    C,
    Cpp,
}

pub fn get_language(extension: &OsStr) -> Language {
    match extension.to_string_lossy().to_string().as_str() {
        "c" | "i" | "h" => Language::C,
        "cc" | "ii" | "cpp" | "cxx" | "c++" | "hh" | "hpp" | "hxx" | "h++" => Language::Cpp,
        _ => unreachable!("unknown file extension language"),
    }
}

pub fn is_code_file(extension: &OsStr) -> bool {
    extension == "c"
        || extension == "i"
        || extension == "cc"
        || extension == "ii"
        || extension == "cpp"
        || extension == "cxx"
        || extension == "c++"
}

pub fn is_header_file(extension: &OsStr) -> bool {
    extension == "h"
        || extension == "hh"
        || extension == "hpp"
        || extension == "hxx"
        || extension == "h++"
}

#[derive(Debug, Clone, DeJson, SerJson)]
pub struct Node {
    pub name: String,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Node>,
}

pub async fn get_ast(
    parser: &mut tree_sitter::Parser,
    maky_ast_path: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> anyhow::Result<(String, Hash, Node, Language)> {
    let maky_ast_path = maky_ast_path.as_ref();
    let path = path.as_ref();

    let language = get_language(
        path.extension()
            .ok_or(anyhow::anyhow!("file extension required"))?,
    );
    let code = read_to_string(path).await?;
    let hash = blake3::hash(code.as_bytes());
    let ast_path = maky_ast_path.join(hash.to_string());

    if ast_path.is_file()
        && let Ok(ast) = read_to_string(&ast_path).await
        && let Ok(ast) = Node::deserialize_json(&ast)
    {
        return Ok((code, hash, ast, language));
    }

    parser.set_language(
        &match language {
            Language::C => tree_sitter_c::LANGUAGE,
            Language::Cpp => tree_sitter_cpp::LANGUAGE,
        }
        .into(),
    )?;

    let tree = parser.parse(&code, None).unwrap();
    let ast = visit_node(tree.root_node());

    Ok((code, hash, ast, language))
}

fn visit_node(node: tree_sitter::Node) -> Node {
    Node {
        name: node.grammar_name().to_string(),
        start: node.start_byte(),
        end: node.end_byte(),
        children: node.children(&mut node.walk()).map(visit_node).collect(),
    }
}
