use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir, read_dir, read_to_string, write},
    io,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use blake3::{hash, Hash};
use clap::{command, Parser};
use pretok::Pretokenizer;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Maky config file
    #[arg(short, long, default_value_t = ("./Maky.toml").to_string())]
    file: String,

    /// Building release
    #[arg(long)]
    release: bool,
}

fn default_binaries() -> PathBuf {
    Path::new("bin").to_path_buf()
}

fn default_sources() -> PathBuf {
    Path::new("src").to_path_buf()
}

fn default_objects() -> PathBuf {
    Path::new("obj").to_path_buf()
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    #[serde(default = "default_binaries")]
    #[serde(alias = "bin")]
    binaries: PathBuf,

    #[serde(default = "default_sources")]
    #[serde(alias = "src")]
    sources: PathBuf,

    #[serde(default = "default_objects")]
    #[serde(alias = "obj")]
    objects: PathBuf,

    #[serde(alias = "inc")]
    includes: Vec<PathBuf>,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let config_path = Path::new(&args.file);
    let config_dir_path = config_path.parent().unwrap_or(Path::new("./"));

    if let Ok(config_file) = read_to_string(config_path) {
        let mut config: Config = toml::from_str(&config_file).unwrap();
        let dir_path = config_dir_path.join("./.maky");

        if !dir_path.is_dir() {
            create_dir(dir_path)?;
        }

        let binaries_dir_path = config_dir_path.join(config.binaries.clone());
        if !binaries_dir_path.is_dir() {
            create_dir(binaries_dir_path)?;
        }

        let sources_dir_path = config_dir_path.join(config.sources.clone());
        if !sources_dir_path.is_dir() {
            create_dir(sources_dir_path)?;
        }

        let objects_dir_path = config_dir_path.join(config.objects.clone());
        if !objects_dir_path.is_dir() {
            create_dir(objects_dir_path.clone())?;
        }

        let hash_hashmap = load_hash_file(config_dir_path);
        let mut new_hash_hashmap = HashMap::new();

        config
            .includes
            .push(config_dir_path.join(config.sources.clone()));

        println!("{:#?}", args);
        println!("{:#?}", config);

        let mut c_h_link = HashMap::new();
        let mut h_c_link = HashMap::new();

        scan_dir(
            &config,
            &config_dir_path.join(config.sources.clone()),
            &mut c_h_link,
            &mut h_c_link,
            &mut new_hash_hashmap,
        )?;

        save_hash_file(config_dir_path, &new_hash_hashmap)?;

        println!("{:#?}", c_h_link);
        println!("{:#?}", h_c_link);
        println!("{:#?}", new_hash_hashmap);

        let mut file_to_compile = HashMap::new();

        let check_file = |file: &PathBuf| -> bool {
            !((file.extension().unwrap_or_default() == "c"
                && hash_hashmap.get(file) == new_hash_hashmap.get(file)
                && objects_dir_path
                    .join(new_hash_hashmap[file].to_hex().as_str())
                    .is_file())
                || (hash_hashmap.get(file) == new_hash_hashmap.get(file)))
        };

        for new_hash in new_hash_hashmap.iter() {
            if let Some(hash) = hash_hashmap.get(new_hash.0) {
                if new_hash.1 == hash
                    && objects_dir_path
                        .join(new_hash.1.to_hex().as_str())
                        .is_file()
                {
                    continue;
                }
            }

            if new_hash.0.extension().unwrap_or_default() == "c" {
                file_to_compile.insert(new_hash.0, new_hash.1);
            } else if let Some(files) = h_c_link.get(new_hash.0) {
                for file in files {
                    if check_file(file) {
                        file_to_compile.insert(file, &new_hash_hashmap[file]);
                    }
                }
            } else {
                c_h_link.iter().for_each(|(file, files)| {
                    if files.contains(new_hash.0) && check_file(file) {
                        file_to_compile.insert(file, &new_hash_hashmap[file]);
                    }
                });
            }
        }

        println!("{:#?}", file_to_compile);

        let mut commands = vec![];

        for file in file_to_compile {
            let mut command = Command::new("gcc");

            if !args.release {
                command.arg("-g").arg("-Wall");
            }

            for include in config.includes.iter() {
                command.arg("-I").arg(include);
            }

            commands.push(
                command
                    .arg("-c")
                    .arg(file.0)
                    .arg("-o")
                    .arg(objects_dir_path.join(file.1.to_hex().as_str()))
                    .spawn()
                    .unwrap(),
            );
        }

        for command in commands.iter_mut() {
            command.wait().unwrap();
        }

        return Ok(());
    }

    eprintln!("No config file found !");

    return Ok(());
}

fn load_hash_file(config_dir_path: &Path) -> HashMap<PathBuf, Hash> {
    let hash_file = read_to_string(config_dir_path.join("./.maky/hash")).unwrap_or_default();
    let mut hash_hashmap = HashMap::new();
    let mut hash_path = Path::new("").to_path_buf();

    for (index, line) in hash_file.lines().enumerate() {
        if index % 2 == 0 {
            hash_path = Path::new(line).to_path_buf();
        } else {
            if let Ok(hash) = Hash::from_str(line) {
                hash_hashmap.insert(hash_path.to_path_buf(), hash);
            }
        }
    }

    hash_hashmap
}

fn save_hash_file(config_dir_path: &Path, hash_hashmap: &HashMap<PathBuf, Hash>) -> io::Result<()> {
    let mut data = vec![];

    for hash in hash_hashmap {
        data.append(
            &mut format!("{}\n{}\n", &hash.0.to_string_lossy(), hash.1.to_hex())
                .as_bytes()
                .to_vec(),
        );
    }

    write(config_dir_path.join("./.maky/hash"), data)
}

fn scan_dir(
    config: &Config,
    dir_path: &Path,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    for entry in read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().unwrap_or_default() == "c" {
            let link_vec = link_c_with_h(&path, c_h_link, hash_hashmap, config.includes.clone());

            for link in link_vec {
                h_c_link
                    .entry(link)
                    .or_insert(HashSet::new())
                    .insert(path.clone());
            }
        } else if path.is_dir() {
            scan_dir(config, &path, c_h_link, h_c_link, hash_hashmap)?;
        }
    }

    return Ok(());
}

fn link_c_with_h(
    path: &Path,
    c_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
    include_path_vec: Vec<PathBuf>,
) -> HashSet<PathBuf> {
    let c_code = read_to_string(path).unwrap();
    let mut c_includes = get_includes(&path, include_path_vec, &c_code);
    let c_prototypes = get_prototypes(&c_code);

    if !hash_hashmap.contains_key(path) {
        hash_hashmap.insert(path.to_path_buf(), hash(c_code.as_bytes()));
    }

    c_h_link.insert(path.to_path_buf(), c_includes.clone());

    c_includes.retain(|c_include| {
        if !c_include.exists() {
            return false;
        }

        let h_code = read_to_string(c_include).unwrap();
        let h_prototypes = get_prototypes(&h_code);

        if !hash_hashmap.contains_key(c_include) {
            hash_hashmap.insert(c_include.to_path_buf(), hash(h_code.as_bytes()));
        }

        return c_prototypes.iter().any(|c_prototype| {
            h_prototypes
                .iter()
                .any(|h_prototype| c_prototype == h_prototype)
        });
    });

    c_includes
}

fn get_includes(path: &Path, include_path_vec: Vec<PathBuf>, code: &String) -> HashSet<PathBuf> {
    let mut include_hashset = HashSet::new();
    let parent_path = path.parent().unwrap_or(Path::new("./")).to_path_buf();

    for line in code.lines() {
        let line = line.trim();

        if line.starts_with("#include") {
            for include in Regex::new("(\"(.*)\"|<(.*)>)").unwrap().captures_iter(line) {
                if let Some(include) = include[0].get(1..include[0].len() - 1) {
                    let path = Path::new(include);

                    if path.is_file() {
                        include_hashset.insert(path.to_path_buf());
                        continue;
                    }

                    let path_with_parent = parent_path.join(path);

                    if path_with_parent.is_file() {
                        include_hashset.insert(path_with_parent.to_path_buf());
                        continue;
                    }

                    for include_path in include_path_vec.clone() {
                        let path = include_path.join(path);

                        if path.is_file() {
                            include_hashset.insert(path.to_path_buf());
                            continue;
                        }
                    }
                }
            }
        }
    }

    include_hashset
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

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use super::*;

    #[test]
    fn linking_c_with_h() {
        let mut c_h_link = HashMap::new();
        let mut hash_hashmap = HashMap::new();

        println!(
            "{:?}",
            link_c_with_h(
                Path::new("./data/src/window/window.c"),
                &mut c_h_link,
                &mut hash_hashmap,
                vec![
                    Path::new("./data/src/").to_path_buf(),
                    Path::new("/usr/include").to_path_buf()
                ]
            )
        );
    }

    #[test]
    fn getting_includes() {
        println!(
            "{:?}",
            get_includes(
                Path::new("window/window.c"),
                vec![
                    Path::new("./data/src/").to_path_buf(),
                    Path::new("/usr/include").to_path_buf()
                ],
                &read_to_string(Path::new("./data/src/window/window.c")).unwrap()
            )
        );
    }

    #[test]
    fn getting_prototypes() {
        println!(
            "{:?}",
            get_prototypes(&read_to_string(Path::new("./data/src/window/window.h")).unwrap())
        );
    }
}
