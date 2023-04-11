use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::{Path, PathBuf},
    process::{Child, Command},
};

use blake3::Hash;
use pretok::Pretokenizer;
use regex::Regex;

use crate::{
    config::{Config, LibConfig},
    get_includes, Args,
};

pub fn link(
    args: &Args,
    config: &Config,
    binaries_dir_path: &Path,
    objects_dir_path: &Path,
    main_hashset: &mut HashSet<PathBuf>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    new_hash_hashmap_clone: &HashMap<PathBuf, Hash>,
) {
    print!("{} file", main_hashset.len());

    if main_hashset.len() > 1 {
        print!("s");
    }

    print!(" to link");

    if main_hashset.len() > 0 {
        println!(" :");
    } else {
        println!(".");
    }

    let h_c_link_filtered = filter_h_c_link(h_c_link);
    let mut commands: Vec<Child> = vec![];

    for main_file in main_hashset.iter() {
        let mut c_file_to_compile = HashSet::new();
        let mut already_explored_h = HashSet::new();

        for h_file in c_h_link[main_file].clone() {
            find_h(config, &h_file, &mut already_explored_h);

            already_explored_h.insert(h_file);
        }

        for h_file in already_explored_h.iter() {
            if let Some(c_file) = h_c_link_filtered.get(h_file) {
                c_file_to_compile.extend(c_file);
            }
        }

        c_file_to_compile.insert(main_file);

        println!("  - {}", &main_file.to_string_lossy());

        let mut command = Command::new(&config.compiler);

        if !args.release {
            command.arg("-g").arg("-Wall");
        }

        for c_file in c_file_to_compile {
            command.arg(objects_dir_path.join(new_hash_hashmap_clone[c_file].to_hex().as_str()));
        }

        for lib in {
            let mut libs: Vec<&LibConfig> = config.libraries.values().collect();

            libs.reverse();
            libs
        } {
            if !lib.regex.is_empty() {
                if !lib
                    .regex
                    .iter()
                    .any(|regex| regex.is_match(&main_file.to_string_lossy()))
                {
                    println!("{}", &main_file.to_string_lossy());
                    continue;
                }
            }

            let mut lib_dir_iter = lib.directories.iter();

            if let Some(lib_dir) = lib_dir_iter.next() {
                command.arg("-L").arg(lib_dir);

                for lib_dir in lib_dir_iter {
                    command.arg("-Wl,-rpath").arg(lib_dir);
                }
            }

            for lib in lib.library.iter() {
                command.arg("-l".to_string() + lib);
            }
        }

        let mut output_file = binaries_dir_path.join(main_file.file_stem().unwrap());

        output_file.set_extension("out");
        commands.push(command.arg("-o").arg(output_file).spawn().unwrap());
    }

    for command in commands.iter_mut() {
        command.wait().unwrap();
    }
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
