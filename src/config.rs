use std::{
    collections, env,
    fs::read_to_string,
    io::{self, stderr},
    path::{Path, PathBuf},
    slice::IterMut,
};

use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use dependency::DependencyConfig;
use features::get_features;
use hashbrown::HashMap;
use lib::LibConfig;
use package::PackageConfig;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use specific::SpecificConfig;
use string_template::Template;

use crate::{
    file::{get_language, Language},
    pkg_config::ParsePkgVersion,
};

pub mod dependency;
pub mod features;
pub mod hash;
pub mod lib;
pub mod package;
pub mod specific;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    pub package: Option<PackageConfig>,

    #[serde(default = "ProjectConfig::default_dependencies")]
    #[serde(alias = "deps")]
    pub dependencies: HashMap<String, DependencyConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "libs")]
    pub libraries: HashMap<String, LibConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "arch")]
    pub arch_specific: HashMap<String, SpecificConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "feat")]
    pub feature_specific: HashMap<String, SpecificConfig>,

    #[serde(default = "ProjectConfig::default_hashmap")]
    #[serde(alias = "os", rename = "os-specific")]
    pub os_specific: HashMap<String, SpecificConfig>,
}

impl ProjectConfig {
    pub fn get_compiler(&self, file: &Path) -> Option<String> {
        file.extension()
            .map(|extention| match get_language(extention) {
                Language::C => self
                    .package
                    .as_ref()
                    .map(|package| package.c_compiler.clone()),
                Language::Cpp => self
                    .package
                    .as_ref()
                    .map(|package| package.cpp_compiler.clone()),
                Language::Other => None,
            })
            .flatten()
    }

    pub fn handle_error(error: io::Error, project_config_path: &Path) -> io::Result<()> {
        if let io::ErrorKind::Other = error.kind() {
            execute!(
                stderr(),
                SetForegroundColor(Color::DarkRed),
                Print("Error ".bold()),
                ResetColor,
                Print(project_config_path.to_string_lossy().bold()),
                Print(" :\n\n".bold()),
                Print(error.to_string()),
            )
        } else {
            execute!(
                stderr(),
                SetForegroundColor(Color::DarkRed),
                Print("Project config not found !\n".bold()),
                ResetColor,
            )
        }
    }

    fn default_dependencies() -> HashMap<String, DependencyConfig> {
        HashMap::new()
    }

    fn default_hashmap<T>() -> HashMap<String, T> {
        HashMap::new()
    }

    fn merge_specific_config(&mut self) {
        let package = self
            .package
            .as_mut()
            .expect("Expected a package section in Maky.toml");
        let mut specific_config = SpecificConfig {
            c_compiler: None,
            cpp_compiler: None,
            binaries: None,
            objects: None,
            sources: None,
            includes: None,
            libraries: None,
        };
        let features = get_features();
        let mut oss = vec![env::consts::OS];

        if env::consts::OS != env::consts::FAMILY {
            oss.push(env::consts::FAMILY);
        }

        let mut inner_merge_specific_config = |selected_specific_config: Option<
            &SpecificConfig,
        >| {
            if let Some(selected_specific_config) = selected_specific_config {
                if let Some(specific_c_compiler) = selected_specific_config.c_compiler.clone() {
                    if let Some(c_compiler) = &mut specific_config.c_compiler {
                        *c_compiler = specific_c_compiler;
                    } else {
                        specific_config.c_compiler = Some(specific_c_compiler);
                    }
                }

                if let Some(specific_cpp_compiler) = selected_specific_config.cpp_compiler.clone() {
                    if let Some(cpp_compiler) = &mut specific_config.cpp_compiler {
                        *cpp_compiler = specific_cpp_compiler;
                    } else {
                        specific_config.cpp_compiler = Some(specific_cpp_compiler);
                    }
                }

                if let Some(specific_binaries) = selected_specific_config.binaries.clone() {
                    if let Some(binaries) = &mut specific_config.binaries {
                        *binaries = specific_binaries;
                    } else {
                        specific_config.binaries = Some(specific_binaries);
                    }
                }

                if let Some(specific_objects) = selected_specific_config.objects.clone() {
                    if let Some(objects) = &mut specific_config.objects {
                        *objects = specific_objects;
                    } else {
                        specific_config.objects = Some(specific_objects);
                    }
                }

                if let Some(specific_sources) = selected_specific_config.sources.clone() {
                    if let Some(sources) = &mut specific_config.sources {
                        sources.extend(specific_sources);
                    } else {
                        specific_config.sources = Some(specific_sources);
                    }
                }

                if let Some(specific_includes) = selected_specific_config.includes.clone() {
                    if let Some(includes) = &mut specific_config.includes {
                        includes.extend(specific_includes);
                    } else {
                        specific_config.includes = Some(specific_includes);
                    }
                }

                if let Some(specific_libraries) = selected_specific_config.libraries.clone() {
                    if let Some(libraries) = &mut specific_config.libraries {
                        libraries.extend(specific_libraries);
                    } else {
                        specific_config.libraries = Some(specific_libraries);
                    }
                }
            }
        };

        inner_merge_specific_config(self.arch_specific.get(env::consts::ARCH));

        for feature in self.feature_specific.keys() {
            if features.contains(feature.as_str()) {
                inner_merge_specific_config(self.feature_specific.get(feature));
            }
        }

        for os in self.os_specific.keys() {
            if oss.contains(&os.as_str()) {
                inner_merge_specific_config(self.os_specific.get(os));
            }
        }

        if let Some(specific_c_compiler) = specific_config.c_compiler {
            package.c_compiler = specific_c_compiler;
        }

        if let Some(specific_cpp_compiler) = specific_config.cpp_compiler {
            package.cpp_compiler = specific_cpp_compiler;
        }

        if let Some(specific_binaries) = specific_config.binaries {
            package.binaries = specific_binaries;
        }

        if let Some(specific_objects) = specific_config.objects {
            package.objects = specific_objects;
        }

        if let Some(specific_sources) = specific_config.sources {
            package.sources.extend(specific_sources);
        }

        if let Some(specific_includes) = specific_config.includes {
            package.includes.extend(specific_includes);
        }

        if let Some(specific_libraries) = specific_config.libraries {
            for (specific_library_name, specific_library_config) in specific_libraries.into_iter() {
                self.libraries
                    .insert(specific_library_name, specific_library_config);
            }
        }

        'main: for library_config in self.libraries.values_mut() {
            let mut library = Vec::new();
            let mut directories = Vec::new();
            let mut includes = Vec::new();

            for (pkg_name, pkg_version) in library_config.pkg_config.iter() {
                if let Ok(pkg_config) = pkg_config::Config::new()
                    .cargo_metadata(false)
                    .env_metadata(false)
                    .parse_version(pkg_version)
                    .probe(pkg_name)
                {
                    library = pkg_config.libs;
                    directories = pkg_config.link_paths;
                    includes = pkg_config.include_paths;
                } else {
                    continue 'main;
                }
            }

            library.extend_from_slice(&library_config.library);
            directories.extend_from_slice(&library_config.directories);
            includes.extend_from_slice(&library_config.includes);

            library_config.library = library;
            library_config.directories = directories;
            library_config.includes = includes;
        }
    }

    pub fn load_without_processing(file_path: &Path) -> io::Result<Self> {
        toml::from_str(&read_to_string(file_path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
    }

    pub fn load(file_path: &Path) -> io::Result<Self> {
        let mut project_config = ProjectConfig::load_without_processing(file_path)?;

        if project_config.package.is_some() {
            project_config.merge_specific_config();

            let package = project_config.package.as_mut().unwrap();
            let template_values = collections::HashMap::from([
                ("os", env::consts::OS),
                ("family", env::consts::FAMILY),
                ("arch", env::consts::ARCH),
            ]);
            let generate_path_variant = |paths: IterMut<PathBuf>| {
                for path in paths {
                    *path = PathBuf::from(
                        Template::new(&path.to_string_lossy()).render(&template_values),
                    );
                }
            };

            generate_path_variant(package.sources.iter_mut());
            generate_path_variant(package.includes.iter_mut());

            for library in project_config.libraries.values_mut() {
                generate_path_variant(library.directories.iter_mut());
                generate_path_variant(library.includes.iter_mut());
            }
        }

        Ok(project_config)
    }
}
