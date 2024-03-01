use std::{
    fmt::Write,
    fs::read_to_string,
    io::{self},
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::Hash;
use pretok::Pretokenizer;

use crate::config::ProjectConfig;

use super::get_includes;

pub fn link(
    project_path: &Path,
    project_config: &ProjectConfig,
    main_vec: &Vec<PathBuf>,
    lib_hashmap: &AHashMap<PathBuf, String>,
    files_to_compile: &AHashMap<PathBuf, Hash>,
    h_c_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
    c_h_link: &AHashMap<PathBuf, AHashSet<PathBuf>>,
) -> io::Result<Vec<(PathBuf, Option<String>, AHashSet<PathBuf>)>> {
    let h_c_link_filtered = filter_h_c_link(h_c_link)?;
    let mut files_to_link = Vec::new();

    let mut link_files = |file: &Path, lib_name_option: Option<String>| -> io::Result<()> {
        let mut file_to_link = AHashSet::new();
        let mut already_explored_h = AHashSet::new();
        let mut need_to_be_link = false;

        for h_file in c_h_link[file].iter() {
            find_h(
                project_path,
                project_config,
                h_file,
                &mut already_explored_h,
            )?;
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

        if files_to_compile.contains_key(file) {
            need_to_be_link = true;
        }

        file_to_link.insert(file.to_path_buf());

        if need_to_be_link {
            files_to_link.push((file.to_path_buf(), lib_name_option, file_to_link));
        }

        Ok(())
    };

    for main_file in main_vec.iter() {
        link_files(main_file, None)?;
    }

    for (lib_file, lib_name) in lib_hashmap.iter() {
        link_files(lib_file, Some(lib_name.clone()))?;
    }

    Ok(files_to_link)
}

fn find_h(
    project_path: &Path,
    project_config: &ProjectConfig,
    h_file: &Path,
    already_explored_h: &mut AHashSet<PathBuf>,
) -> io::Result<()> {
    if !already_explored_h.contains(h_file) {
        already_explored_h.insert(h_file.to_path_buf());

        let code = read_to_string(h_file)?;
        let includes = get_includes(h_file, project_path, &project_config.includes, &code);

        for include in includes {
            find_h(project_path, project_config, &include, already_explored_h)?;
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
