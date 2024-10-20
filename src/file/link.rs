use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use blake3::Hash;
use hashbrown::{HashMap, HashSet};
use indoc::formatdoc;
use pretok::{Pretoken, Pretokenizer};

use crate::config::ProjectConfig;

use super::get_includes;

pub fn link(
    project_path: &Path,
    project_config: &ProjectConfig,
    main_hashmap: &HashMap<PathBuf, Option<String>>,
    lib_hashmap: &HashMap<PathBuf, Option<String>>,
    files_to_compile: &HashMap<PathBuf, Hash>,
    h_c_link: &HashMap<PathBuf, HashSet<PathBuf>>,
    c_h_link: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> anyhow::Result<Vec<(PathBuf, bool, Option<String>, HashSet<PathBuf>)>> {
    let h_c_link_filtered = filter_h_c_link(h_c_link)?;
    let mut files_to_link = Vec::new();

    let mut link_files = |file: &Path,
                          is_library: bool,
                          name_option: Option<String>|
     -> anyhow::Result<()> {
        let mut file_to_link = HashSet::new();
        let mut already_explored_h = HashSet::new();
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
                    if file != c_file && main_hashmap.contains_key(c_file) {
                        return Err(anyhow!("{}", formatdoc!(
                            "Header file functions are defined in {} and {} containing each one a main function.
                            You must define these functions in separate code files, not containing a main function.
                            This is to avoid problems with redefining the main function during compilation.",
                            file.to_string_lossy(),
                            c_file.to_string_lossy()
                        )));
                    }

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
            files_to_link.push((file.to_path_buf(), is_library, name_option, file_to_link));
        }

        Ok(())
    };

    for (main_file, main_name) in main_hashmap {
        link_files(main_file, false, main_name.clone())?;
    }

    for (lib_file, lib_name) in lib_hashmap.iter() {
        link_files(lib_file, true, lib_name.clone())?;
    }

    Ok(files_to_link)
}

fn find_h(
    project_path: &Path,
    project_config: &ProjectConfig,
    h_file: &Path,
    already_explored_h: &mut HashSet<PathBuf>,
) -> anyhow::Result<()> {
    if !already_explored_h.contains(h_file) {
        already_explored_h.insert(h_file.to_path_buf());

        let code = read_to_string(h_file)?;
        let includes = get_includes(
            h_file,
            project_path,
            &project_config.package.as_ref().unwrap().includes,
            &code,
        );

        for include in includes {
            find_h(project_path, project_config, &include, already_explored_h)?;
        }
    }

    Ok(())
}

fn filter_h_c_link(
    h_c_link: &HashMap<PathBuf, HashSet<PathBuf>>,
) -> anyhow::Result<HashMap<PathBuf, HashSet<PathBuf>>> {
    let mut link_filtered: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

    for (h_file, c_files) in h_c_link.iter() {
        let h_code = read_to_string(h_file)?;
        let h_prototypes = get_h_prototypes(&h_code)?;

        for c_file in c_files {
            let c_code = read_to_string(c_file)?;
            let c_prototypes = get_c_prototypes(&c_code)?;

            if !c_prototypes.is_disjoint(&h_prototypes) {
                link_filtered
                    .entry(h_file.to_path_buf())
                    .or_insert(HashSet::new())
                    .insert(c_file.to_path_buf());
            }
        }
    }

    Ok(link_filtered)
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Declaration {
    Class(String),
    Function(String),
}

fn get_h_prototypes(code: &str) -> anyhow::Result<HashSet<Declaration>> {
    let mut prototype_hashset = HashSet::new();
    let mut pretokenizer = Pretokenizer::new(code);
    let mut pretoken_vec: Vec<&str> = Vec::new();
    let mut classname_vec = Vec::new();

    while let Some(Pretoken { s, .. }) = pretokenizer.next() {
        match s {
            "class" => {
                let classname = to_result(pretokenizer.next())?.s;

                classname_vec.push(classname);
                prototype_hashset.insert(Declaration::Class(classname.to_string()));
            }
            "};" => {
                classname_vec.pop();
            }
            _ => {
                if s.ends_with(");") {
                    let prototype = {
                        let mut parenthesis_count = count_parenthesis(s);
                        let mut prototype: Vec<String> = vec![s.to_string()];

                        while parenthesis_count != 0 {
                            let s = to_result(pretoken_vec.pop())?.to_string();

                            clear_prototype(&s, &mut prototype);

                            parenthesis_count += count_parenthesis(&s);
                            prototype.push(s);
                        }

                        if prototype.last().unwrap().starts_with("(") {
                            prototype.push(to_result(pretoken_vec.pop())?.to_string());
                        }

                        prototype.reverse();
                        prototype[0] = prototype[0]
                            .strip_prefix("*")
                            .unwrap_or(&prototype[0])
                            .to_string();
                        prototype.join("")
                    };

                    prototype_hashset.insert(Declaration::Function(
                        classname_vec
                            .last()
                            .map_or(String::new(), |classname| classname.to_string() + "::")
                            + &prototype,
                    ));

                    continue;
                }

                pretoken_vec.push(s);
            }
        }
    }

    Ok(prototype_hashset)
}

fn get_c_prototypes(code: &str) -> anyhow::Result<HashSet<Declaration>> {
    let mut prototype_hashset = HashSet::new();
    let mut pretokenizer = Pretokenizer::new(code);
    let mut pretoken_vec: Vec<&str> = Vec::new();

    while let Some(Pretoken { s, .. }) = pretokenizer.next() {
        if s.ends_with("{") {
            let prototype = {
                let mut parenthesis_count = 0;
                let mut prototype: Vec<String> = Vec::new();

                loop {
                    let s = to_result(pretoken_vec.pop())?.to_string();

                    clear_prototype(&s, &mut prototype);

                    parenthesis_count += count_parenthesis(&s);
                    prototype.push(s);

                    if parenthesis_count == 0 {
                        break;
                    }
                }

                if prototype.last().unwrap().starts_with("(") {
                    prototype.push(to_result(pretoken_vec.pop())?.to_string());
                }

                prototype.reverse();

                if let "if" | "else" | "for" | "while" | "switch" | "}" | "NULL" | "=" | "\\" =
                    prototype[0].as_str()
                {
                    continue;
                }

                prototype[0] = prototype[0]
                    .strip_prefix("*")
                    .unwrap_or(&prototype[0])
                    .to_string();
                prototype.join("").split(".").last().unwrap().to_string() + ";"
            };

            prototype_hashset.insert(Declaration::Function(prototype));

            continue;
        }

        pretoken_vec.push(s);
    }

    Ok(prototype_hashset)
}

fn clear_prototype(s: &String, prototype: &mut Vec<String>) {
    if let Some(last_s) = prototype.last_mut() {
        if (last_s.ends_with(",") || last_s.ends_with(")") || last_s.ends_with(");"))
            && !(s.ends_with(",") || s.ends_with("("))
        {
            last_s.retain(|char| {
                if let ',' | '*' | '[' | ']' | '(' | ')' | ';' = char {
                    true
                } else {
                    false
                }
            });
        }
    }
}

fn count_parenthesis(code: &str) -> i32 {
    code.chars().fold(0, |accumulator, char| {
        accumulator
            + match char {
                ')' => 1,
                '(' => -1,
                _ => 0,
            }
    })
}

fn to_result<T>(option: Option<T>) -> anyhow::Result<T> {
    option.ok_or(anyhow!("There is no more token to parse."))
}
