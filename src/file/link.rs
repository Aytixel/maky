use std::{
    fmt::Write,
    fs::read_to_string,
    io::{self},
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::Hash;
use hyperscan::{pattern, BlockDatabase, Builder, Matching};
use once_cell::sync::Lazy;
use pretok::Pretokenizer;

use crate::config::ProjectConfig;

use super::get_includes;

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
        if let Some(last) = import.split("//@import ").last() {
            *import = last.to_string();
        }
    }

    return imports;
}

pub fn link(
    project_config: &ProjectConfig,
    main_hashset: &AHashSet<PathBuf>,
    files_to_compile: &AHashMap<PathBuf, Hash>,
    h_c_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
    c_h_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
) -> io::Result<Vec<(PathBuf, AHashSet<PathBuf>)>> {
    let h_c_link_filtered = filter_h_c_link(h_c_link)?;
    let mut files_to_link = vec![];

    for main_file in main_hashset.iter() {
        let mut file_to_link = AHashSet::new();
        let mut already_explored_h = AHashSet::new();
        let mut need_to_be_link = false;

        for h_file in c_h_link[main_file].iter() {
            find_h(project_config, h_file, &mut already_explored_h)?;
        }

        for h_file in already_explored_h.iter() {
            if let Some(c_files) = h_c_link_filtered.get(h_file) {
                for c_file in c_files {
                    if files_to_compile.contains_key(c_file) {
                        need_to_be_link = true;
                    }

                    file_to_link.insert(c_file.to_path_buf());
                }
            }
        }

        if files_to_compile.contains_key(main_file) {
            need_to_be_link = true;
        }

        file_to_link.insert(main_file.to_path_buf());

        if need_to_be_link {
            files_to_link.push((main_file.to_path_buf(), file_to_link));
        }
    }

    Ok(files_to_link)
}

fn find_h(
    project_config: &ProjectConfig,
    h_file: &Path,
    already_explored_h: &mut AHashSet<PathBuf>,
) -> io::Result<()> {
    if !already_explored_h.contains(h_file) {
        already_explored_h.insert(h_file.to_path_buf());

        let code = read_to_string(h_file)?;
        let includes = get_includes(h_file, &project_config.includes, &code);

        for include in includes {
            find_h(project_config, &include, already_explored_h)?;
        }
    }

    Ok(())
}

fn filter_h_c_link(
    h_c_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
) -> io::Result<AHashMap<PathBuf, AHashSet<PathBuf>>> {
    let mut link_filtered: AHashMap<PathBuf, AHashSet<PathBuf>> = AHashMap::new();

    for (h_file, c_files) in h_c_link.iter() {
        let h_code = read_to_string(h_file)?;
        let h_prototypes = get_prototypes(&h_code);

        for c_file in c_files {
            let c_code = read_to_string(c_file)?;
            let c_prototypes = get_prototypes(&c_code);

            if !c_prototypes.is_disjoint(&h_prototypes) {
                link_filtered
                    .entry(h_file.to_path_buf())
                    .or_insert(AHashSet::new())
                    .insert(c_file.to_path_buf());
            }
        }
    }

    Ok(link_filtered)
}

fn get_prototypes(code: &str) -> AHashSet<String> {
    let mut function_prototype_hashset = AHashSet::new();
    let mut function_prototype = String::new();
    let mut in_function = false;

    for pretoken in Pretokenizer::new(code) {
        match pretoken.s {
            "extern" | "inline" | "static" => {
                in_function = true;
            }
            _ => {
                if in_function {
                    if !pretoken.s.starts_with('{') {
                        function_prototype += " ";

                        if pretoken.s.ends_with(';') {
                            function_prototype += &pretoken.s[..pretoken.s.len() - 1];
                        } else {
                            function_prototype += pretoken.s;
                        }
                    }

                    if pretoken.s.ends_with(';') || pretoken.s.starts_with('{') {
                        let mut formatted_function_prototype = String::new();
                        let mut last_char = ' ';

                        for char in function_prototype.chars() {
                            match char {
                                ' ' | '\t' | '\n' | '\r' => {
                                    if last_char != ' ' {
                                        formatted_function_prototype += " ";
                                        last_char = ' ';
                                    }
                                }
                                '*' => {
                                    if last_char != ' ' {
                                        formatted_function_prototype += " *";
                                    } else {
                                        formatted_function_prototype += "*";
                                    }

                                    last_char = '*';
                                }
                                _ => {
                                    if last_char == '*' {
                                        formatted_function_prototype += " ";
                                        formatted_function_prototype.write_char(char).unwrap();
                                        last_char = char;
                                    } else {
                                        formatted_function_prototype.write_char(char).unwrap();
                                        last_char = char;
                                    }
                                }
                            }
                        }
                        function_prototype_hashset.insert(formatted_function_prototype);
                        function_prototype = String::new();
                        in_function = false;
                    }
                }
            }
        }
    }

    function_prototype_hashset
}
