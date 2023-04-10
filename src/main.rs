mod compile;
mod config;

use std::{
    collections::{HashMap, HashSet},
    fs::{create_dir, read_dir, read_to_string},
    io,
    path::{Path, PathBuf},
};

use blake3::{hash, Hash};
use clap::{command, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    compile::compile,
    config::{load_hash_file, save_hash_file},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
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
pub struct Config {
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

        config
            .includes
            .push(config_dir_path.join(config.sources.clone()));

        println!("{:#?}", args);
        println!("{:#?}", config);

        let mut hash_hashmap = load_hash_file(config_dir_path);
        let mut new_hash_hashmap = HashMap::new();
        let mut h_h_link = HashMap::new();
        let mut h_c_link = HashMap::new();

        scan_dir(
            &config,
            &config_dir_path.join(config.sources.clone()),
            &mut h_h_link,
            &mut h_c_link,
            &mut new_hash_hashmap,
        )?;

        save_hash_file(config_dir_path, &new_hash_hashmap)?;

        compile(
            &args,
            &config,
            &objects_dir_path,
            &mut h_h_link,
            &mut h_c_link,
            &mut hash_hashmap,
            &mut new_hash_hashmap,
        )?;

        return Ok(());
    }

    eprintln!("No config file found !");

    return Ok(());
}

fn scan_dir(
    config: &Config,
    dir_path: &Path,
    h_h_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    h_c_link: &mut HashMap<PathBuf, HashSet<PathBuf>>,
    hash_hashmap: &mut HashMap<PathBuf, Hash>,
) -> io::Result<()> {
    for entry in read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let extension = path.extension().unwrap_or_default();

            if extension == "c" || extension == "h" {
                let code = read_to_string(&path).unwrap();
                let includes = get_includes(&path, config.includes.clone(), &code);

                hash_hashmap.insert(path.to_path_buf(), hash(code.as_bytes()));

                for include in includes {
                    if extension == "c" {
                        h_c_link
                            .entry(include)
                            .or_insert(HashSet::new())
                            .insert(path.clone());
                    } else {
                        h_h_link
                            .entry(include)
                            .or_insert(HashSet::new())
                            .insert(path.clone());
                    }
                }
            }
        } else if path.is_dir() {
            scan_dir(config, &path, h_h_link, h_c_link, hash_hashmap)?;
        }
    }

    return Ok(());
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

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;

    use super::*;

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
}
