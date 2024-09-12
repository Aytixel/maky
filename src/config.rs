use std::{
    collections, env,
    fs::{read_to_string, write},
    io::{self, stderr},
    path::{Path, PathBuf},
    slice::IterMut,
};

use blake3::Hash;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_with::{formats::PreferOne, serde_as, OneOrMany};
use string_template::Template;

use crate::{
    file::{get_language, Language},
    pkg_config::ParsePkgVersion,
};

pub trait LoadConfig {
    fn load(path: &Path) -> io::Result<Self>
    where
        Self: Sized;
}

pub trait SaveConfig {
    fn save(&self, path: &Path) -> io::Result<()>;
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    #[serde(default = "ProjectConfig::default_c_compiler")]
    #[serde(alias = "cc")]
    pub c_compiler: String,

    #[serde(default = "ProjectConfig::default_cpp_compiler")]
    #[serde(alias = "cxx")]
    pub cpp_compiler: String,

    #[serde(alias = "std")]
    pub standard: Option<String>,

    #[serde(default = "ProjectConfig::default_binaries")]
    #[serde(alias = "bin")]
    pub binaries: PathBuf,

    #[serde(default = "ProjectConfig::default_objects")]
    #[serde(alias = "obj")]
    pub objects: PathBuf,

    #[serde(default = "ProjectConfig::default_sources")]
    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub sources: Vec<PathBuf>,

    #[serde(default = "ProjectConfig::default_includes")]
    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub includes: Vec<PathBuf>,

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
    #[serde(alias = "os")]
    pub os_specific: HashMap<String, SpecificConfig>,
}

impl ProjectConfig {
    pub fn get_compiler(&self, file: &Path) -> Option<String> {
        file.extension()
            .map(|extention| match get_language(extention) {
                Language::C => Some(self.c_compiler.clone()),
                Language::Cpp => Some(self.cpp_compiler.clone()),
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

    fn default_c_compiler() -> String {
        "gcc".to_string()
    }

    fn default_cpp_compiler() -> String {
        "g++".to_string()
    }

    fn default_binaries() -> PathBuf {
        Path::new("bin").to_path_buf()
    }

    fn default_objects() -> PathBuf {
        Path::new("obj").to_path_buf()
    }

    fn default_sources() -> Vec<PathBuf> {
        vec![Path::new("src").to_path_buf()]
    }

    fn default_includes() -> Vec<PathBuf> {
        vec![Path::new("include").to_path_buf()]
    }

    fn default_dependencies() -> HashMap<String, DependencyConfig> {
        HashMap::new()
    }

    fn default_hashmap<T>() -> HashMap<String, T> {
        HashMap::new()
    }

    fn merge_specific_config(&mut self) {
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
            self.c_compiler = specific_c_compiler;
        }

        if let Some(specific_cpp_compiler) = specific_config.cpp_compiler {
            self.cpp_compiler = specific_cpp_compiler;
        }

        if let Some(specific_binaries) = specific_config.binaries {
            self.binaries = specific_binaries;
        }

        if let Some(specific_objects) = specific_config.objects {
            self.objects = specific_objects;
        }

        if let Some(specific_sources) = specific_config.sources {
            self.sources.extend(specific_sources);
        }

        if let Some(specific_includes) = specific_config.includes {
            self.includes.extend(specific_includes);
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
}

impl LoadConfig for ProjectConfig {
    fn load(file_path: &Path) -> io::Result<Self> {
        let mut project_config: ProjectConfig = toml::from_str(&read_to_string(file_path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;

        project_config.merge_specific_config();

        let template_values = collections::HashMap::from([
            ("os", env::consts::OS),
            ("family", env::consts::FAMILY),
            ("arch", env::consts::ARCH),
        ]);
        let generate_path_variant = |paths: IterMut<PathBuf>| {
            for path in paths {
                *path =
                    PathBuf::from(Template::new(&path.to_string_lossy()).render(&template_values));
            }
        };

        generate_path_variant(project_config.sources.iter_mut());
        generate_path_variant(project_config.includes.iter_mut());

        for library in project_config.libraries.values_mut() {
            generate_path_variant(library.directories.iter_mut());
            generate_path_variant(library.includes.iter_mut());
        }

        Ok(project_config)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DependencyConfig {
    Local { path: PathBuf },
    Git { git: String, rev: Option<String> },
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SpecificConfig {
    #[serde(alias = "cc")]
    pub c_compiler: Option<String>,

    #[serde(alias = "cxx")]
    pub cpp_compiler: Option<String>,

    #[serde(alias = "bin")]
    pub binaries: Option<PathBuf>,

    #[serde(alias = "obj")]
    pub objects: Option<PathBuf>,

    #[serde(alias = "src")]
    #[serde_as(deserialize_as = "Option<OneOrMany<_, PreferOne>>")]
    pub sources: Option<Vec<PathBuf>>,

    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "Option<OneOrMany<_, PreferOne>>")]
    pub includes: Option<Vec<PathBuf>>,

    #[serde(alias = "libs")]
    pub libraries: Option<HashMap<String, LibConfig>>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibConfig {
    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "lib")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub library: Vec<String>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "dir")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub directories: Vec<PathBuf>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "inc")]
    #[serde_as(deserialize_as = "OneOrMany<_, PreferOne>")]
    pub includes: Vec<PathBuf>,

    #[serde(default = "LibConfig::default_hashmap")]
    #[serde(alias = "pkg")]
    pub pkg_config: HashMap<String, String>,
}

impl LibConfig {
    fn default_vec<T>() -> Vec<T> {
        Vec::new()
    }

    fn default_hashmap<T>() -> HashMap<String, T> {
        HashMap::new()
    }
}

impl LoadConfig for HashMap<PathBuf, Hash> {
    fn load(project_path: &Path) -> io::Result<Self> {
        let hash_file = read_to_string(project_path.join(".maky/hash"))?;
        let mut hash_hashmap = HashMap::new();
        let mut hash_path = Path::new("");

        for (index, line) in hash_file.lines().enumerate() {
            if index % 2 == 0 {
                hash_path = Path::new(line);
            } else {
                if let Ok(hash) = Hash::from_hex(line) {
                    hash_hashmap.insert(project_path.join(hash_path), hash);
                }
            }
        }

        Ok(hash_hashmap)
    }
}

impl SaveConfig for HashMap<PathBuf, Hash> {
    fn save(&self, project_path: &Path) -> io::Result<()> {
        let mut data = Vec::new();

        for hash in self {
            data.append(
                &mut format!(
                    "{}\n{}\n",
                    &hash
                        .0
                        .strip_prefix(project_path)
                        .unwrap_or(hash.0)
                        .to_string_lossy(),
                    hash.1.to_hex()
                )
                .as_bytes()
                .to_vec(),
            );
        }

        write(project_path.join("./.maky/hash"), data)
    }
}

fn get_features() -> HashSet<&'static str> {
    let mut features = HashSet::new();

    if cfg!(target_feature = "adx") {
        features.insert("adx");
    }
    if cfg!(target_feature = "aes") {
        features.insert("aes");
    }
    if cfg!(target_feature = "avx") {
        features.insert("avx");
    }
    if cfg!(target_feature = "avx2") {
        features.insert("avx2");
    }
    if cfg!(target_feature = "bmi1") {
        features.insert("bmi1");
    }
    if cfg!(target_feature = "bmi2") {
        features.insert("bmi2");
    }
    if cfg!(target_feature = "fma") {
        features.insert("fma");
    }
    if cfg!(target_feature = "fxsr") {
        features.insert("fxsr");
    }
    if cfg!(target_feature = "lzcnt") {
        features.insert("lzcnt");
    }
    if cfg!(target_feature = "pclmulqdq") {
        features.insert("pclmulqdq");
    }
    if cfg!(target_feature = "popcnt") {
        features.insert("popcnt");
    }
    if cfg!(target_feature = "rdrand") {
        features.insert("rdrand");
    }
    if cfg!(target_feature = "rdseed") {
        features.insert("rdseed");
    }
    if cfg!(target_feature = "sha") {
        features.insert("sha");
    }
    if cfg!(target_feature = "sse") {
        features.insert("sse");
    }
    if cfg!(target_feature = "sse2") {
        features.insert("sse2");
    }
    if cfg!(target_feature = "sse3") {
        features.insert("sse3");
    }
    if cfg!(target_feature = "sse4.1") {
        features.insert("sse4.1");
    }
    if cfg!(target_feature = "sse4.2") {
        features.insert("sse4.2");
    }
    if cfg!(target_feature = "ssse3") {
        features.insert("ssse3");
    }
    if cfg!(target_feature = "xsave") {
        features.insert("xsave");
    }
    if cfg!(target_feature = "xsavec") {
        features.insert("xsavec");
    }
    if cfg!(target_feature = "xsaveopt") {
        features.insert("xsaveopt");
    }
    if cfg!(target_feature = "xsaves") {
        features.insert("xsaves");
    }
    if cfg!(target_feature = "bf16") {
        features.insert("bf16");
    }
    if cfg!(target_feature = "bti") {
        features.insert("bti");
    }
    if cfg!(target_feature = "crc") {
        features.insert("crc");
    }
    if cfg!(target_feature = "dit") {
        features.insert("dit");
    }
    if cfg!(target_feature = "dotprod") {
        features.insert("dotprod");
    }
    if cfg!(target_feature = "dpb") {
        features.insert("dpb");
    }
    if cfg!(target_feature = "dpb2") {
        features.insert("dpb2");
    }
    if cfg!(target_feature = "f32mm") {
        features.insert("f32mm");
    }
    if cfg!(target_feature = "f64mm") {
        features.insert("f64mm");
    }
    if cfg!(target_feature = "fcma") {
        features.insert("fcma");
    }
    if cfg!(target_feature = "fhm") {
        features.insert("fhm");
    }
    if cfg!(target_feature = "flagm") {
        features.insert("flagm");
    }
    if cfg!(target_feature = "fp16") {
        features.insert("fp16");
    }
    if cfg!(target_feature = "frintts") {
        features.insert("frintts");
    }
    if cfg!(target_feature = "i8mm") {
        features.insert("i8mm");
    }
    if cfg!(target_feature = "jsconv") {
        features.insert("jsconv");
    }
    if cfg!(target_feature = "lse") {
        features.insert("lse");
    }
    if cfg!(target_feature = "lor") {
        features.insert("lor");
    }
    if cfg!(target_feature = "mte") {
        features.insert("mte");
    }
    if cfg!(target_feature = "neon") {
        features.insert("neon");
    }
    if cfg!(target_feature = "pan") {
        features.insert("pan");
    }
    if cfg!(target_feature = "paca") {
        features.insert("paca");
    }
    if cfg!(target_feature = "pacg") {
        features.insert("pacg");
    }
    if cfg!(target_feature = "pmuv3") {
        features.insert("pmuv3");
    }
    if cfg!(target_feature = "rand") {
        features.insert("rand");
    }
    if cfg!(target_feature = "ras") {
        features.insert("ras");
    }
    if cfg!(target_feature = "rcpc") {
        features.insert("rcpc");
    }
    if cfg!(target_feature = "rcpc2") {
        features.insert("rcpc2");
    }
    if cfg!(target_feature = "rdm") {
        features.insert("rdm");
    }
    if cfg!(target_feature = "sb") {
        features.insert("sb");
    }
    if cfg!(target_feature = "sha2") {
        features.insert("sha2");
    }
    if cfg!(target_feature = "sha3") {
        features.insert("sha3");
    }
    if cfg!(target_feature = "sm4") {
        features.insert("sm4");
    }
    if cfg!(target_feature = "spe") {
        features.insert("spe");
    }
    if cfg!(target_feature = "ssbs") {
        features.insert("ssbs");
    }
    if cfg!(target_feature = "sve") {
        features.insert("sve");
    }
    if cfg!(target_feature = "sve2") {
        features.insert("sve2");
    }
    if cfg!(target_feature = "sve2-aes") {
        features.insert("sve2-aes");
    }
    if cfg!(target_feature = "sve2-sm4") {
        features.insert("sve2-sm4");
    }
    if cfg!(target_feature = "sve2-sha3") {
        features.insert("sve2-sha3");
    }
    if cfg!(target_feature = "sve2-bitperm") {
        features.insert("sve2-bitperm");
    }
    if cfg!(target_feature = "tme") {
        features.insert("tme");
    }
    if cfg!(target_feature = "vh") {
        features.insert("vh");
    }
    if cfg!(target_feature = "simd128") {
        features.insert("simd128");
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if is_x86_feature_detected!("aes") {
            features.insert("aes");
        }
        if is_x86_feature_detected!("pclmulqdq") {
            features.insert("pclmulqdq");
        }
        if is_x86_feature_detected!("rdrand") {
            features.insert("rdrand");
        }
        if is_x86_feature_detected!("rdseed") {
            features.insert("rdseed");
        }
        if is_x86_feature_detected!("tsc") {
            features.insert("tsc");
        }
        if is_x86_feature_detected!("mmx") {
            features.insert("mmx");
        }
        if is_x86_feature_detected!("sse") {
            features.insert("sse");
        }
        if is_x86_feature_detected!("sse2") {
            features.insert("sse2");
        }
        if is_x86_feature_detected!("sse3") {
            features.insert("sse3");
        }
        if is_x86_feature_detected!("ssse3") {
            features.insert("ssse3");
        }
        if is_x86_feature_detected!("sse4.1") {
            features.insert("sse4.1");
        }
        if is_x86_feature_detected!("sse4.2") {
            features.insert("sse4.2");
        }
        if is_x86_feature_detected!("sse4a") {
            features.insert("sse4a");
        }
        if is_x86_feature_detected!("sha") {
            features.insert("sha");
        }
        if is_x86_feature_detected!("avx") {
            features.insert("avx");
        }
        if is_x86_feature_detected!("avx2") {
            features.insert("avx2");
        }
        if is_x86_feature_detected!("avx512f") {
            features.insert("avx512f");
        }
        if is_x86_feature_detected!("avx512cd") {
            features.insert("avx512cd");
        }
        if is_x86_feature_detected!("avx512er") {
            features.insert("avx512er");
        }
        if is_x86_feature_detected!("avx512pf") {
            features.insert("avx512pf");
        }
        if is_x86_feature_detected!("avx512bw") {
            features.insert("avx512bw");
        }
        if is_x86_feature_detected!("avx512dq") {
            features.insert("avx512dq");
        }
        if is_x86_feature_detected!("avx512vl") {
            features.insert("avx512vl");
        }
        if is_x86_feature_detected!("avx512ifma") {
            features.insert("avx512ifma");
        }
        if is_x86_feature_detected!("avx512vbmi") {
            features.insert("avx512vbmi");
        }
        if is_x86_feature_detected!("avx512vpopcntdq") {
            features.insert("avx512vpopcntdq");
        }
        if is_x86_feature_detected!("avx512vbmi2") {
            features.insert("avx512vbmi2");
        }
        if is_x86_feature_detected!("gfni") {
            features.insert("avx512gfni");
        }
        if is_x86_feature_detected!("vaes") {
            features.insert("avx512vaes");
        }
        if is_x86_feature_detected!("vpclmulqdq") {
            features.insert("avx512vpclmulqdq");
        }
        if is_x86_feature_detected!("avx512vnni") {
            features.insert("avx512vnni");
        }
        if is_x86_feature_detected!("avx512bitalg") {
            features.insert("avx512bitalg");
        }
        if is_x86_feature_detected!("avx512bf16") {
            features.insert("avx512bf16");
        }
        if is_x86_feature_detected!("avx512vp2intersect") {
            features.insert("avx512vp2intersect");
        }
        if is_x86_feature_detected!("f16c") {
            features.insert("f16c");
        }
        if is_x86_feature_detected!("fma") {
            features.insert("fma");
        }
        if is_x86_feature_detected!("bmi1") {
            features.insert("bmi1");
        }
        if is_x86_feature_detected!("bmi2") {
            features.insert("bmi2");
        }
        if is_x86_feature_detected!("lzcnt") {
            features.insert("lzcnt");
        }
        if is_x86_feature_detected!("tbm") {
            features.insert("tbm");
        }
        if is_x86_feature_detected!("popcnt") {
            features.insert("popcnt");
        }
        if is_x86_feature_detected!("fxsr") {
            features.insert("fxsr");
        }
        if is_x86_feature_detected!("xsave") {
            features.insert("xsave");
        }
        if is_x86_feature_detected!("xsaveopt") {
            features.insert("xsaveopt");
        }
        if is_x86_feature_detected!("xsaves") {
            features.insert("xsaves");
        }
        if is_x86_feature_detected!("xsavec") {
            features.insert("xsavec");
        }
        if is_x86_feature_detected!("cmpxchg16b") {
            features.insert("cmpxchg16b");
        }
        if is_x86_feature_detected!("adx") {
            features.insert("adx");
        }
        if is_x86_feature_detected!("rtm") {
            features.insert("rtm");
        }
        if is_x86_feature_detected!("abm") {
            features.insert("abm");
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        use std::arch::is_aarch64_feature_detected;

        if is_aarch64_feature_detected!("neon") {
            features.insert("neon");
        }
        if is_aarch64_feature_detected!("pmull") {
            features.insert("pmull");
        }
        if is_aarch64_feature_detected!("fp") {
            features.insert("fp");
        }
        if is_aarch64_feature_detected!("fp16") {
            features.insert("fp16");
        }
        if is_aarch64_feature_detected!("sve") {
            features.insert("sve");
        }
        if is_aarch64_feature_detected!("crc") {
            features.insert("crc");
        }
        if is_aarch64_feature_detected!("lse") {
            features.insert("lse");
        }
        if is_aarch64_feature_detected!("lse2") {
            features.insert("lse2");
        }
        if is_aarch64_feature_detected!("rdm") {
            features.insert("rdm");
        }
        if is_aarch64_feature_detected!("rcpc") {
            features.insert("rcpc");
        }
        if is_aarch64_feature_detected!("rcpc2") {
            features.insert("rcpc2");
        }
        if is_aarch64_feature_detected!("dotprod") {
            features.insert("dotprod");
        }
        if is_aarch64_feature_detected!("tme") {
            features.insert("tme");
        }
        if is_aarch64_feature_detected!("fhm") {
            features.insert("fhm");
        }
        if is_aarch64_feature_detected!("dit") {
            features.insert("dit");
        }
        if is_aarch64_feature_detected!("flagm") {
            features.insert("flagm");
        }
        if is_aarch64_feature_detected!("ssbs") {
            features.insert("ssbs");
        }
        if is_aarch64_feature_detected!("sb") {
            features.insert("sb");
        }
        if is_aarch64_feature_detected!("paca") {
            features.insert("paca");
        }
        if is_aarch64_feature_detected!("pacg") {
            features.insert("pacg");
        }
        if is_aarch64_feature_detected!("dpb") {
            features.insert("dpb");
        }
        if is_aarch64_feature_detected!("dpb2") {
            features.insert("dpb2");
        }
        if is_aarch64_feature_detected!("sve2") {
            features.insert("sve2");
        }
        if is_aarch64_feature_detected!("sve2-aes") {
            features.insert("sve2-aes");
        }
        if is_aarch64_feature_detected!("sve2-sm4") {
            features.insert("sve2-sm4");
        }
        if is_aarch64_feature_detected!("sve2-sha3") {
            features.insert("sve2-sha3");
        }
        if is_aarch64_feature_detected!("sve2-bitperm") {
            features.insert("sve2-bitperm");
        }
        if is_aarch64_feature_detected!("frintts") {
            features.insert("frintts");
        }
        if is_aarch64_feature_detected!("i8mm") {
            features.insert("i8mm");
        }
        if is_aarch64_feature_detected!("f32mm") {
            features.insert("f32mm");
        }
        if is_aarch64_feature_detected!("f64mm") {
            features.insert("f64mm");
        }
        if is_aarch64_feature_detected!("bf16") {
            features.insert("bf16");
        }
        if is_aarch64_feature_detected!("rand") {
            features.insert("rand");
        }
        if is_aarch64_feature_detected!("bti") {
            features.insert("bti");
        }
        if is_aarch64_feature_detected!("mte") {
            features.insert("mte");
        }
        if is_aarch64_feature_detected!("jsconv") {
            features.insert("jsconv");
        }
        if is_aarch64_feature_detected!("fcma") {
            features.insert("fcma");
        }
        if is_aarch64_feature_detected!("aes") {
            features.insert("aes");
        }
        if is_aarch64_feature_detected!("sha2") {
            features.insert("sha2");
        }
        if is_aarch64_feature_detected!("sha3") {
            features.insert("sha3");
        }
        if is_aarch64_feature_detected!("sm4") {
            features.insert("sm4");
        }
        if is_aarch64_feature_detected!("asimd") {
            features.insert("asimd");
        }
        if is_aarch64_feature_detected!("ras") {
            features.insert("ras");
        }
        if is_aarch64_feature_detected!("v8.1a") {
            features.insert("v8.1a");
        }
        if is_aarch64_feature_detected!("v8.2a") {
            features.insert("v8.2a");
        }
        if is_aarch64_feature_detected!("v8.3a") {
            features.insert("v8.3a");
        }
        if is_aarch64_feature_detected!("v8.4a") {
            features.insert("v8.4a");
        }
        if is_aarch64_feature_detected!("v8.5a") {
            features.insert("v8.5a");
        }
        if is_aarch64_feature_detected!("v8.6a") {
            features.insert("v8.6a");
        }
        if is_aarch64_feature_detected!("v8.7a") {
            features.insert("v8.7a");
        }
    }

    features
}
