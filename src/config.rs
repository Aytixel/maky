use std::{
    collections::HashMap,
    env,
    fs::{read_to_string, write},
    io,
    path::{Path, PathBuf},
};

use ahash::{AHashMap, AHashSet};
use blake3::Hash;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub trait LoadConfig {
    fn load(path: &Path) -> io::Result<Self>
    where
        Self: Sized;
}

pub trait SaveConfig {
    fn save(&self, path: &Path) -> io::Result<()>;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default = "Config::default_bool")]
    pub release: bool,
}

impl Config {
    fn default_bool() -> bool {
        false
    }
}

impl LoadConfig for Config {
    fn load(project_path: &Path) -> io::Result<Self> {
        toml::from_str(&read_to_string(project_path.join(".maky/config.toml"))?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))
    }
}

impl SaveConfig for Config {
    fn save(&self, project_path: &Path) -> io::Result<()> {
        write(
            project_path.join("./.maky/config.toml"),
            toml::to_string_pretty(self)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    #[serde(default = "ProjectConfig::default_compiler")]
    #[serde(alias = "cc")]
    pub compiler: String,

    #[serde(default = "ProjectConfig::default_binaries")]
    #[serde(alias = "bin")]
    pub binaries: PathBuf,

    #[serde(default = "ProjectConfig::default_objects")]
    #[serde(alias = "obj")]
    pub objects: PathBuf,

    #[serde(default = "ProjectConfig::default_sources")]
    #[serde(alias = "src")]
    pub sources: Vec<PathBuf>,

    #[serde(default = "ProjectConfig::default_includes")]
    #[serde(alias = "inc")]
    pub includes: Vec<PathBuf>,

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
    fn default_compiler() -> String {
        "gcc".to_string()
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
        vec![]
    }

    fn default_hashmap<T>() -> HashMap<String, T> {
        HashMap::new()
    }

    fn merge_specific_config(
        &mut self,
        archs: AHashSet<&str>,
        features: AHashSet<&str>,
        oss: AHashSet<&str>,
    ) {
        let mut specific_config = SpecificConfig {
            compiler: None,
            binaries: None,
            objects: None,
            sources: None,
            includes: None,
            libraries: None,
        };

        for arch in archs {
            if let Some(arch_specific_config) = self.arch_specific.get(arch) {
                if let Some(specific_compiler) = arch_specific_config.compiler.clone() {
                    if let Some(compiler) = &mut specific_config.compiler {
                        *compiler = specific_compiler;
                    } else {
                        specific_config.compiler = Some(specific_compiler);
                    }
                }

                if let Some(specific_binaries) = arch_specific_config.binaries.clone() {
                    if let Some(binaries) = &mut specific_config.binaries {
                        *binaries = specific_binaries;
                    } else {
                        specific_config.binaries = Some(specific_binaries);
                    }
                }

                if let Some(specific_objects) = arch_specific_config.objects.clone() {
                    if let Some(objects) = &mut specific_config.objects {
                        *objects = specific_objects;
                    } else {
                        specific_config.objects = Some(specific_objects);
                    }
                }

                if let Some(specific_sources) = arch_specific_config.sources.clone() {
                    if let Some(sources) = &mut specific_config.sources {
                        sources.extend(specific_sources);
                    } else {
                        specific_config.sources = Some(specific_sources);
                    }
                }

                if let Some(specific_includes) = arch_specific_config.includes.clone() {
                    if let Some(includes) = &mut specific_config.includes {
                        includes.extend(specific_includes);
                    } else {
                        specific_config.includes = Some(specific_includes);
                    }
                }

                if let Some(specific_libraries) = arch_specific_config.libraries.clone() {
                    if let Some(libraries) = &mut specific_config.libraries {
                        libraries.extend(specific_libraries);
                    } else {
                        specific_config.libraries = Some(specific_libraries);
                    }
                }
            }
        }

        for feature in features {
            if let Some(feature_specific_config) = self.feature_specific.get(feature) {
                if let Some(specific_compiler) = feature_specific_config.compiler.clone() {
                    if let Some(compiler) = &mut specific_config.compiler {
                        *compiler = specific_compiler;
                    } else {
                        specific_config.compiler = Some(specific_compiler);
                    }
                }

                if let Some(specific_binaries) = feature_specific_config.binaries.clone() {
                    if let Some(binaries) = &mut specific_config.binaries {
                        *binaries = specific_binaries;
                    } else {
                        specific_config.binaries = Some(specific_binaries);
                    }
                }

                if let Some(specific_objects) = feature_specific_config.objects.clone() {
                    if let Some(objects) = &mut specific_config.objects {
                        *objects = specific_objects;
                    } else {
                        specific_config.objects = Some(specific_objects);
                    }
                }

                if let Some(specific_sources) = feature_specific_config.sources.clone() {
                    if let Some(sources) = &mut specific_config.sources {
                        sources.extend(specific_sources);
                    } else {
                        specific_config.sources = Some(specific_sources);
                    }
                }

                if let Some(specific_includes) = feature_specific_config.includes.clone() {
                    if let Some(includes) = &mut specific_config.includes {
                        includes.extend(specific_includes);
                    } else {
                        specific_config.includes = Some(specific_includes);
                    }
                }

                if let Some(specific_libraries) = feature_specific_config.libraries.clone() {
                    if let Some(libraries) = &mut specific_config.libraries {
                        libraries.extend(specific_libraries);
                    } else {
                        specific_config.libraries = Some(specific_libraries);
                    }
                }
            }
        }

        for os in oss {
            if let Some(os_specific_config) = self.os_specific.get(os) {
                if let Some(specific_compiler) = os_specific_config.compiler.clone() {
                    if let Some(compiler) = &mut specific_config.compiler {
                        *compiler = specific_compiler;
                    } else {
                        specific_config.compiler = Some(specific_compiler);
                    }
                }

                if let Some(specific_binaries) = os_specific_config.binaries.clone() {
                    if let Some(binaries) = &mut specific_config.binaries {
                        *binaries = specific_binaries;
                    } else {
                        specific_config.binaries = Some(specific_binaries);
                    }
                }

                if let Some(specific_objects) = os_specific_config.objects.clone() {
                    if let Some(objects) = &mut specific_config.objects {
                        *objects = specific_objects;
                    } else {
                        specific_config.objects = Some(specific_objects);
                    }
                }

                if let Some(specific_sources) = os_specific_config.sources.clone() {
                    if let Some(sources) = &mut specific_config.sources {
                        sources.extend(specific_sources);
                    } else {
                        specific_config.sources = Some(specific_sources);
                    }
                }

                if let Some(specific_includes) = os_specific_config.includes.clone() {
                    if let Some(includes) = &mut specific_config.includes {
                        includes.extend(specific_includes);
                    } else {
                        specific_config.includes = Some(specific_includes);
                    }
                }

                if let Some(specific_libraries) = os_specific_config.libraries.clone() {
                    if let Some(libraries) = &mut specific_config.libraries {
                        libraries.extend(specific_libraries);
                    } else {
                        specific_config.libraries = Some(specific_libraries);
                    }
                }
            }
        }

        if let Some(specific_compiler) = specific_config.compiler {
            self.compiler = specific_compiler;
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
            self.libraries.extend(specific_libraries);
        }
    }
}

impl LoadConfig for ProjectConfig {
    fn load(file_path: &Path) -> io::Result<Self> {
        let mut project_config: ProjectConfig = toml::from_str(&read_to_string(file_path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?;
        let mut archs = AHashSet::new();
        let mut oss = AHashSet::new();

        archs.insert(env::consts::ARCH);

        oss.insert(env::consts::OS);
        oss.insert(env::consts::FAMILY);

        project_config.merge_specific_config(archs, get_features(), oss);

        Ok(project_config)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpecificConfig {
    #[serde(alias = "cc")]
    pub compiler: Option<String>,

    #[serde(alias = "bin")]
    pub binaries: Option<PathBuf>,

    #[serde(alias = "obj")]
    pub objects: Option<PathBuf>,

    #[serde(alias = "src")]
    pub sources: Option<Vec<PathBuf>>,

    #[serde(alias = "inc")]
    pub includes: Option<Vec<PathBuf>>,

    #[serde(alias = "libs")]
    pub libraries: Option<HashMap<String, LibConfig>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibConfig {
    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "reg")]
    #[serde(with = "serde_regex")]
    pub regex: Vec<Regex>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "lib")]
    pub library: Vec<String>,

    #[serde(default = "LibConfig::default_vec")]
    #[serde(alias = "dir")]
    pub directories: Vec<PathBuf>,
}

impl LibConfig {
    fn default_vec<T>() -> Vec<T> {
        vec![]
    }
}

impl LoadConfig for AHashMap<PathBuf, Hash> {
    fn load(project_path: &Path) -> io::Result<Self> {
        let hash_file = read_to_string(project_path.join(".maky/hash"))?;
        let mut hash_hashmap = AHashMap::new();
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

impl SaveConfig for AHashMap<PathBuf, Hash> {
    fn save(&self, project_path: &Path) -> io::Result<()> {
        let mut data = vec![];

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

fn get_features<'a>() -> AHashSet<&'a str> {
    let mut features = AHashSet::new();

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
