use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::{Path, PathBuf},
};

use blake3::Hash;
use pretok::Pretokenizer;
use regex::Regex;

use crate::{config::Config, get_includes};

pub fn link(
    config: &Config,
    main_hashset: &HashSet<PathBuf>,
    files_to_compile: &HashMap<PathBuf, Hash>,
    h_c_link: &HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> Vec<(PathBuf, HashSet<PathBuf>)> {
    let h_c_link_filtered = filter_h_c_link(h_c_link);
    let mut files_to_link = vec![];

    for main_file in main_hashset.iter() {
        let mut file_to_link = HashSet::new();
        let mut already_explored_h = HashSet::new();
        let mut need_to_be_link = false;

        for h_file in c_h_link[main_file].clone() {
            find_h(config, &h_file, &mut already_explored_h);

            already_explored_h.insert(h_file);
        }

        for h_file in already_explored_h.iter() {
            if let Some(c_files) = h_c_link_filtered.get(h_file) {
                for c_file in c_files {
                    if files_to_compile.keys().any(|path| path == c_file) {
                        need_to_be_link = true;
                    }

                    file_to_link.insert(c_file.to_path_buf());
                }
            }
        }

        if files_to_compile.keys().any(|path| path == main_file) {
            need_to_be_link = true;
        }

        file_to_link.insert(main_file.to_path_buf());

        if need_to_be_link {
            files_to_link.push((main_file.to_path_buf(), file_to_link));
        }
    }

    files_to_link
}

fn find_h(config: &Config, h_file: &Path, already_explored_h: &mut HashSet<PathBuf>) {
    if !already_explored_h.contains(h_file) {
        already_explored_h.insert(h_file.to_path_buf());

        let code = read_to_string(&h_file).unwrap();
        let includes = get_includes(&h_file, config.includes.clone(), &code);

        for include in includes {
            find_h(config, &include, already_explored_h);
        }
    }
}

fn filter_h_c_link(
    h_c_link: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> HashMap<PathBuf, HashSet<PathBuf>> {
    let mut link_filtered: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

    for (h_file, c_files) in h_c_link.iter() {
        let h_code = read_to_string(&h_file).unwrap();
        let h_prototypes = get_prototypes(&h_code);

        for c_file in c_files {
            let c_code = read_to_string(&c_file).unwrap();
            let c_prototypes = get_prototypes(&c_code);

            if c_prototypes.iter().any(|c_prototype| {
                h_prototypes
                    .iter()
                    .any(|h_prototype| c_prototype == h_prototype)
            }) {
                link_filtered
                    .entry(h_file.to_path_buf())
                    .or_insert(HashSet::new())
                    .insert(c_file.to_path_buf());
            }
        }
    }

    link_filtered
}

fn get_prototypes(code: &String) -> HashSet<String> {
    let mut function_prototype_hashset = HashSet::new();
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
                        function_prototype = Regex::new(r"[ \t\n\r]*\*")
                            .unwrap()
                            .replace_all(&function_prototype[1..], " *")
                            .to_string();
                        function_prototype = Regex::new(r"\*[ \t\n\r]*")
                            .unwrap()
                            .replace_all(&function_prototype, "* ")
                            .to_string();
                        function_prototype = Regex::new(r"[ \t\n\r]+")
                            .unwrap()
                            .replace_all(&function_prototype, " ")
                            .to_string();
                        function_prototype_hashset.insert(function_prototype);
                        function_prototype = String::new();
                        in_function = false;
                    }
                }
            }
        }
    }

    function_prototype_hashset
}
