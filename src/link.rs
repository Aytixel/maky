use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::{Path, PathBuf},
    process::{Child, Command},
};

use blake3::Hash;
use pretok::Pretokenizer;
use regex::Regex;

use crate::{get_includes, Args, Config};

pub fn link(
    args: &Args,
    config: &Config,
    binaries_dir_path: &Path,
    objects_dir_path: &Path,
    main_hashset: &mut HashSet<PathBuf>,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    new_hash_hashmap_clone: &HashMap<PathBuf, Hash>,
) {
    let mut h_c_link_filtered = filter_h_c_link(h_c_link);
    let mut commands: Vec<Child> = vec![];

    println!("{:#?}", h_c_link_filtered.len());

    for main_file in main_hashset.iter() {
        println!("{}", &main_file.to_string_lossy());

        let mut file_to_compile = HashSet::new();
        let mut already_explored = HashSet::new();

        for h_file in c_h_link[main_file].clone() {
            find_c(
                config,
                objects_dir_path,
                &h_file,
                h_h_link,
                &mut h_c_link_filtered,
                new_hash_hashmap_clone,
                &mut file_to_compile,
                &mut already_explored,
            );
        }

        drop(already_explored);

        println!("{:#?}", file_to_compile);

        let mut command = Command::new("gcc");

        if !args.release {
            command.arg("-g").arg("-Wall");
        }

        for file in file_to_compile {
            command.arg(file);
        }

        println!(
            "{:?}",
            command
                .arg("-o")
                .arg(binaries_dir_path.join(main_file.file_name().unwrap()))
        );

        /*
        commands.push(
            command
                .arg("-o")
                .arg(binaries_dir_path.join(main_file.file_name().unwrap()))
                .spawn()
                .unwrap(),
        );
        */
    }

    for command in commands.iter_mut() {
        command.wait().unwrap();
    }
}

fn find_c(
    config: &Config,
    objects_dir_path: &Path,
    h_file: &Path,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    new_hash_hashmap_clone: &HashMap<PathBuf, Hash>,
    file_to_compile: &mut HashSet<PathBuf>,
    already_explored: &mut HashSet<PathBuf>,
) {
    if !already_explored.contains(h_file) {
        already_explored.insert(h_file.to_path_buf());

        if let Some(h_files) = h_h_link.get(h_file).cloned() {
            for h_file in h_files {
                find_c(
                    config,
                    objects_dir_path,
                    &h_file,
                    h_h_link,
                    h_c_link,
                    new_hash_hashmap_clone,
                    file_to_compile,
                    already_explored,
                );
            }
        } else if let Some(c_files) = h_c_link.get(h_file).cloned() {
            for c_file in c_files {
                file_to_compile.insert(
                    objects_dir_path.join(new_hash_hashmap_clone[&c_file].to_hex().as_str()),
                );
            }
        }
    }
}

fn filter_h_c_link(
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
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
