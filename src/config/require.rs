use std::{cmp, collections::HashSet, env, fmt, sync::LazyLock};

use serde::Deserialize;

#[derive(Deserialize, Default, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub(in crate::config) enum Require {
    Value(String),
    Or {
        or: Vec<Require>,
    },
    And {
        and: Vec<Require>,
    },
    #[default]
    None,
}

impl Require {
    pub fn has_requirements(&self) -> bool {
        match self {
            Require::Value(requirement) => TARGET_REQUIREMENTS.contains(requirement),
            Require::Or { or } => or.iter().any(Require::has_requirements),
            Require::And { and } => and.iter().all(Require::has_requirements),
            Require::None => true,
        }
    }
}

impl cmp::PartialOrd for Require {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl cmp::Ord for Require {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        match (self, other) {
            (Require::None, _) => cmp::Ordering::Greater,
            (_, Require::None) => cmp::Ordering::Less,
            _ => cmp::Ordering::Equal,
        }
    }
}

impl fmt::Display for Require {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Require::Value(requirement) => write!(f, "{requirement}"),
            Require::Or { or } => {
                if or.is_empty() {
                    write!(f, "nothing")
                } else if or.len() == 1 {
                    write!(f, "{:?}", or[0])
                } else {
                    write!(
                        f,
                        "({})",
                        or.iter()
                            .map(Require::to_string)
                            .collect::<Vec<_>>()
                            .join(" || ")
                    )
                }
            }
            Require::And { and } => {
                if and.is_empty() {
                    write!(f, "nothing")
                } else if and.len() == 1 {
                    write!(f, "{:?}", and[0])
                } else {
                    write!(
                        f,
                        "({})",
                        and.iter()
                            .map(Require::to_string)
                            .collect::<Vec<_>>()
                            .join(" && ")
                    )
                }
            }
            Require::None => write!(f, "nothing"),
        }
    }
}

impl fmt::Debug for Require {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Require::Value(requirement) => write!(f, "{requirement}"),
            Require::Or { or } => {
                if or.is_empty() {
                    write!(f, "nothing")
                } else if or.len() == 1 {
                    write!(f, "{:?}", or[0])
                } else {
                    write!(
                        f,
                        "({})",
                        or.iter()
                            .map(Require::to_string)
                            .collect::<Vec<_>>()
                            .join(" || ")
                    )
                }
            }
            Require::And { and } => {
                if and.is_empty() {
                    write!(f, "nothing")
                } else if and.len() == 1 {
                    write!(f, "{:?}", and[0])
                } else {
                    write!(
                        f,
                        "({})",
                        and.iter()
                            .map(Require::to_string)
                            .collect::<Vec<_>>()
                            .join(" && ")
                    )
                }
            }
            Require::None => write!(f, "nothing"),
        }
    }
}

const TARGET_REQUIREMENTS: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut targets = HashSet::from([
        format!("os:{}", env::consts::OS),
        format!("family:{}", env::consts::FAMILY),
        format!("arch:{}", env::consts::ARCH),
    ]);

    // https://doc.rust-lang.org/std/arch/index.html
    #[cfg(any(target_arch = "aarch64", target_arch = "arm64ec"))]
    {
        use std::arch::is_aarch64_feature_detected;

        if is_aarch64_feature_detected!("aes") {
            targets.insert("feature:aes".to_string());
            targets.insert("feature:pmull".to_string());
        }
        if is_aarch64_feature_detected!("asimd") {
            targets.insert("feature:asimd".to_string());
        }
        if is_aarch64_feature_detected!("neon") {
            targets.insert("feature:neon".to_string());
        }
        if is_aarch64_feature_detected!("bf16") {
            targets.insert("feature:bf16".to_string());
        }
        if is_aarch64_feature_detected!("bti") {
            targets.insert("feature:bti".to_string());
        }
        if is_aarch64_feature_detected!("crc") {
            targets.insert("feature:crc".to_string());
        }
        if is_aarch64_feature_detected!("cssc") {
            targets.insert("feature:cssc".to_string());
        }
        if is_aarch64_feature_detected!("dit") {
            targets.insert("feature:dit".to_string());
        }
        if is_aarch64_feature_detected!("dotprod") {
            targets.insert("feature:dotprod".to_string());
        }
        if is_aarch64_feature_detected!("dpb") {
            targets.insert("feature:dpb".to_string());
        }
        if is_aarch64_feature_detected!("dpb2") {
            targets.insert("feature:dpb2".to_string());
        }
        if is_aarch64_feature_detected!("ecv") {
            targets.insert("feature:ecv".to_string());
        }
        if is_aarch64_feature_detected!("f32mm") {
            targets.insert("feature:f32mm".to_string());
        }
        if is_aarch64_feature_detected!("f64mm") {
            targets.insert("feature:f64mm".to_string());
        }
        if is_aarch64_feature_detected!("faminmax") {
            targets.insert("feature:faminmax".to_string());
        }
        if is_aarch64_feature_detected!("fcma") {
            targets.insert("feature:fcma".to_string());
        }
        if is_aarch64_feature_detected!("fhm") {
            targets.insert("feature:fhm".to_string());
        }
        if is_aarch64_feature_detected!("flagm") {
            targets.insert("feature:flagm".to_string());
        }
        if is_aarch64_feature_detected!("flagm2") {
            targets.insert("feature:flagm2".to_string());
        }
        if is_aarch64_feature_detected!("fp") {
            targets.insert("feature:fp".to_string());
        }
        if is_aarch64_feature_detected!("fp16") {
            targets.insert("feature:fp16".to_string());
        }
        if is_aarch64_feature_detected!("fp8") {
            targets.insert("feature:fp8".to_string());
        }
        if is_aarch64_feature_detected!("fp8dot2") {
            targets.insert("feature:fp8dot2".to_string());
        }
        if is_aarch64_feature_detected!("fp8dot4") {
            targets.insert("feature:fp8dot4".to_string());
        }
        if is_aarch64_feature_detected!("fp8fma") {
            targets.insert("feature:fp8fma".to_string());
        }
        if is_aarch64_feature_detected!("fpmr") {
            targets.insert("feature:fpmr".to_string());
        }
        if is_aarch64_feature_detected!("frintts") {
            targets.insert("feature:frintts".to_string());
        }
        if is_aarch64_feature_detected!("hbc") {
            targets.insert("feature:hbc".to_string());
        }
        if is_aarch64_feature_detected!("i8mm") {
            targets.insert("feature:i8mm".to_string());
        }
        if is_aarch64_feature_detected!("jsconv") {
            targets.insert("feature:jsconv".to_string());
        }
        if is_aarch64_feature_detected!("lse") {
            targets.insert("feature:lse".to_string());
        }
        if is_aarch64_feature_detected!("lse128") {
            targets.insert("feature:lse128".to_string());
        }
        if is_aarch64_feature_detected!("lse2") {
            targets.insert("feature:lse2".to_string());
        }
        if is_aarch64_feature_detected!("lut") {
            targets.insert("feature:lut".to_string());
        }
        if is_aarch64_feature_detected!("mops") {
            targets.insert("feature:mops".to_string());
        }
        if is_aarch64_feature_detected!("mte") {
            targets.insert("feature:mte".to_string());
        }
        if is_aarch64_feature_detected!("paca") {
            targets.insert("feature:paca".to_string());
        }
        if is_aarch64_feature_detected!("pacg") {
            targets.insert("feature:pacg".to_string());
        }
        if is_aarch64_feature_detected!("pauth-lr") {
            targets.insert("feature:pauth-lr".to_string());
        }
        if is_aarch64_feature_detected!("pmull") {
            targets.insert("feature:pmull".to_string());
        }
        if is_aarch64_feature_detected!("rand") {
            targets.insert("feature:rand".to_string());
        }
        if is_aarch64_feature_detected!("rcpc") {
            targets.insert("feature:rcpc".to_string());
        }
        if is_aarch64_feature_detected!("rcpc2") {
            targets.insert("feature:rcpc2".to_string());
        }
        if is_aarch64_feature_detected!("rcpc3") {
            targets.insert("feature:rcpc3".to_string());
        }
        if is_aarch64_feature_detected!("rdm") {
            targets.insert("feature:rdm".to_string());
        }
        if is_aarch64_feature_detected!("sb") {
            targets.insert("feature:sb".to_string());
        }
        if is_aarch64_feature_detected!("sha2") {
            targets.insert("feature:sha2".to_string());
        }
        if is_aarch64_feature_detected!("sha3") {
            targets.insert("feature:sha3".to_string());
        }
        if is_aarch64_feature_detected!("sm4") {
            targets.insert("feature:sm4".to_string());
        }
        if is_aarch64_feature_detected!("sme") {
            targets.insert("feature:sme".to_string());
        }
        if is_aarch64_feature_detected!("sme-b16b16") {
            targets.insert("feature:sme-b16b16".to_string());
        }
        if is_aarch64_feature_detected!("sme-f16f16") {
            targets.insert("feature:sme-f16f16".to_string());
        }
        if is_aarch64_feature_detected!("sme-f64f64") {
            targets.insert("feature:sme-f64f64".to_string());
        }
        if is_aarch64_feature_detected!("sme-f8f16") {
            targets.insert("feature:sme-f8f16".to_string());
        }
        if is_aarch64_feature_detected!("sme-f8f32") {
            targets.insert("feature:sme-f8f32".to_string());
        }
        if is_aarch64_feature_detected!("sme-fa64") {
            targets.insert("feature:sme-fa64".to_string());
        }
        if is_aarch64_feature_detected!("sme-i16i64") {
            targets.insert("feature:sme-i16i64".to_string());
        }
        if is_aarch64_feature_detected!("sme-lutv2") {
            targets.insert("feature:sme-lutv2".to_string());
        }
        if is_aarch64_feature_detected!("sme2") {
            targets.insert("feature:sme2".to_string());
        }
        if is_aarch64_feature_detected!("sme2p1") {
            targets.insert("feature:sme2p1".to_string());
        }
        if is_aarch64_feature_detected!("ssbs") {
            targets.insert("feature:ssbs".to_string());
        }
        if is_aarch64_feature_detected!("ssve-fp8dot2") {
            targets.insert("feature:ssve-fp8dot2".to_string());
        }
        if is_aarch64_feature_detected!("ssve-fp8dot4") {
            targets.insert("feature:ssve-fp8dot4".to_string());
        }
        if is_aarch64_feature_detected!("ssve-fp8fma") {
            targets.insert("feature:ssve-fp8fma".to_string());
        }
        if is_aarch64_feature_detected!("sve") {
            targets.insert("feature:sve".to_string());
        }
        if is_aarch64_feature_detected!("sve-b16b16") {
            targets.insert("feature:sve-b16b16".to_string());
        }
        if is_aarch64_feature_detected!("sve2") {
            targets.insert("feature:sve2".to_string());
        }
        if is_aarch64_feature_detected!("sve2-aes") {
            targets.insert("feature:sve2-aes".to_string());
        }
        if is_aarch64_feature_detected!("sve2-bitperm") {
            targets.insert("feature:sve2-bitperm".to_string());
        }
        if is_aarch64_feature_detected!("sve2-sha3") {
            targets.insert("feature:sve2-sha3".to_string());
        }
        if is_aarch64_feature_detected!("sve2-sm4") {
            targets.insert("feature:sve2-sm4".to_string());
        }
        if is_aarch64_feature_detected!("sve2p1") {
            targets.insert("feature:sve2p1".to_string());
        }
        if is_aarch64_feature_detected!("tme") {
            targets.insert("feature:tme".to_string());
        }
        if is_aarch64_feature_detected!("wfxt") {
            targets.insert("feature:wfxt".to_string());
        }
    }

    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    {
        use std::arch::is_riscv_feature_detected;

        if is_riscv_feature_detected!("rv32e") {
            targets.insert("feature:rv32e".to_string());
        }
        if is_riscv_feature_detected!("rv32i") {
            targets.insert("feature:rv32i".to_string());
        }
        if is_riscv_feature_detected!("rv64i") {
            targets.insert("feature:rv64i".to_string());
        }
        if is_riscv_feature_detected!("a") {
            targets.insert("feature:a".to_string());
        }
        if is_riscv_feature_detected!("b") {
            targets.insert("feature:b".to_string());
        }
        if is_riscv_feature_detected!("zba") {
            targets.insert("feature:zba".to_string());
        }
        if is_riscv_feature_detected!("zbb") {
            targets.insert("feature:zbb".to_string());
        }
        if is_riscv_feature_detected!("zbc") {
            targets.insert("feature:zbc".to_string());
        }
        if is_riscv_feature_detected!("zbs") {
            targets.insert("feature:zbs".to_string());
        }
        if is_riscv_feature_detected!("c") {
            targets.insert("feature:c".to_string());
        }
        if is_riscv_feature_detected!("d") {
            targets.insert("feature:d".to_string());
        }
        if is_riscv_feature_detected!("f") {
            targets.insert("feature:f".to_string());
        }
        if is_riscv_feature_detected!("m") {
            targets.insert("feature:m".to_string());
        }
        if is_riscv_feature_detected!("q") {
            targets.insert("feature:q".to_string());
        }
        if is_riscv_feature_detected!("v") {
            targets.insert("feature:v".to_string());
        }
        if is_riscv_feature_detected!("zicntr") {
            targets.insert("feature:zicntr".to_string());
        }
        if is_riscv_feature_detected!("zicsr") {
            targets.insert("feature:zicsr".to_string());
        }
        if is_riscv_feature_detected!("zifencei") {
            targets.insert("feature:zifencei".to_string());
        }
        if is_riscv_feature_detected!("zihintpause") {
            targets.insert("feature:zihintpause".to_string());
        }
        if is_riscv_feature_detected!("zihpm") {
            targets.insert("feature:zihpm".to_string());
        }
        if is_riscv_feature_detected!("zk") {
            targets.insert("feature:zk".to_string());
        }
        if is_riscv_feature_detected!("zbkb") {
            targets.insert("feature:zbkb".to_string());
        }
        if is_riscv_feature_detected!("zbkc") {
            targets.insert("feature:zbkc".to_string());
        }
        if is_riscv_feature_detected!("zbkx") {
            targets.insert("feature:zbkx".to_string());
        }
        if is_riscv_feature_detected!("zkn") {
            targets.insert("feature:zkn".to_string());
        }
        if is_riscv_feature_detected!("zknd") {
            targets.insert("feature:zknd".to_string());
        }
        if is_riscv_feature_detected!("zkne") {
            targets.insert("feature:zkne".to_string());
        }
        if is_riscv_feature_detected!("zknh") {
            targets.insert("feature:zknh".to_string());
        }
        if is_riscv_feature_detected!("zkr") {
            targets.insert("feature:zkr".to_string());
        }
        if is_riscv_feature_detected!("zks") {
            targets.insert("feature:zks".to_string());
        }
        if is_riscv_feature_detected!("zksed") {
            targets.insert("feature:zksed".to_string());
        }
        if is_riscv_feature_detected!("zksh") {
            targets.insert("feature:zksh".to_string());
        }
        if is_riscv_feature_detected!("zkt") {
            targets.insert("feature:zkt".to_string());
        }
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        use std::arch::is_x86_feature_detected;

        if is_x86_feature_detected!("aes") {
            targets.insert("feature:aes".to_string());
        }
        if is_x86_feature_detected!("pclmulqdq") {
            targets.insert("feature:pclmulqdq".to_string());
        }
        if is_x86_feature_detected!("rdrand") {
            targets.insert("feature:rdrand".to_string());
        }
        if is_x86_feature_detected!("rdseed") {
            targets.insert("feature:rdseed".to_string());
        }
        if is_x86_feature_detected!("tsc") {
            targets.insert("feature:tsc".to_string());
        }
        if is_x86_feature_detected!("mmx") {
            targets.insert("feature:mmx".to_string());
        }
        if is_x86_feature_detected!("sse") {
            targets.insert("feature:sse".to_string());
        }
        if is_x86_feature_detected!("sse2") {
            targets.insert("feature:sse2".to_string());
        }
        if is_x86_feature_detected!("sse3") {
            targets.insert("feature:sse3".to_string());
        }
        if is_x86_feature_detected!("ssse3") {
            targets.insert("feature:ssse3".to_string());
        }
        if is_x86_feature_detected!("sse4.1") {
            targets.insert("feature:sse4.1".to_string());
        }
        if is_x86_feature_detected!("sse4.2") {
            targets.insert("feature:sse4.2".to_string());
        }
        if is_x86_feature_detected!("sse4a") {
            targets.insert("feature:sse4a".to_string());
        }
        if is_x86_feature_detected!("sha") {
            targets.insert("feature:sha".to_string());
        }
        if is_x86_feature_detected!("avx") {
            targets.insert("feature:avx".to_string());
        }
        if is_x86_feature_detected!("avx2") {
            targets.insert("feature:avx2".to_string());
        }
        if is_x86_feature_detected!("avx512f") {
            targets.insert("feature:avx512f".to_string());
        }
        if is_x86_feature_detected!("avx512cd") {
            targets.insert("feature:avx512cd".to_string());
        }
        if is_x86_feature_detected!("avx512er") {
            targets.insert("feature:avx512er".to_string());
        }
        if is_x86_feature_detected!("avx512pf") {
            targets.insert("feature:avx512pf".to_string());
        }
        if is_x86_feature_detected!("avx512bw") {
            targets.insert("feature:avx512bw".to_string());
        }
        if is_x86_feature_detected!("avx512dq") {
            targets.insert("feature:avx512dq".to_string());
        }
        if is_x86_feature_detected!("avx512vl") {
            targets.insert("feature:avx512vl".to_string());
        }
        if is_x86_feature_detected!("avx512ifma") {
            targets.insert("feature:avx512ifma".to_string());
        }
        if is_x86_feature_detected!("avx512vbmi") {
            targets.insert("feature:avx512vbmi".to_string());
        }
        if is_x86_feature_detected!("avx512vpopcntdq") {
            targets.insert("feature:avx512vpopcntdq".to_string());
        }
        if is_x86_feature_detected!("avx512vbmi2") {
            targets.insert("feature:avx512vbmi2".to_string());
        }
        if is_x86_feature_detected!("gfni") {
            targets.insert("feature:gfni".to_string());
        }
        if is_x86_feature_detected!("vaes") {
            targets.insert("feature:vaes".to_string());
        }
        if is_x86_feature_detected!("vpclmulqdq") {
            targets.insert("feature:vpclmulqdq".to_string());
        }
        if is_x86_feature_detected!("avx512vnni") {
            targets.insert("feature:avx512vnni".to_string());
        }
        if is_x86_feature_detected!("avx512bitalg") {
            targets.insert("feature:avx512bitalg".to_string());
        }
        if is_x86_feature_detected!("avx512bf16") {
            targets.insert("feature:avx512bf16".to_string());
        }
        if is_x86_feature_detected!("avx512vp2intersect") {
            targets.insert("feature:avx512vp2intersect".to_string());
        }
        if is_x86_feature_detected!("avx512fp16") {
            targets.insert("feature:avx512fp16".to_string());
        }
        if is_x86_feature_detected!("f16c") {
            targets.insert("feature:f16c".to_string());
        }
        if is_x86_feature_detected!("fma") {
            targets.insert("feature:fma".to_string());
        }
        if is_x86_feature_detected!("bmi1") {
            targets.insert("feature:bmi1".to_string());
        }
        if is_x86_feature_detected!("bmi2") {
            targets.insert("feature:bmi2".to_string());
        }
        if is_x86_feature_detected!("abm") {
            targets.insert("feature:abm".to_string());
        }
        if is_x86_feature_detected!("lzcnt") {
            targets.insert("feature:lzcnt".to_string());
        }
        if is_x86_feature_detected!("tbm") {
            targets.insert("feature:tbm".to_string());
        }
        if is_x86_feature_detected!("popcnt") {
            targets.insert("feature:popcnt".to_string());
        }
        if is_x86_feature_detected!("fxsr") {
            targets.insert("feature:fxsr".to_string());
        }
        if is_x86_feature_detected!("xsave") {
            targets.insert("feature:xsave".to_string());
        }
        if is_x86_feature_detected!("xsaveopt") {
            targets.insert("feature:xsaveopt".to_string());
        }
        if is_x86_feature_detected!("xsaves") {
            targets.insert("feature:xsaves".to_string());
        }
        if is_x86_feature_detected!("xsavec") {
            targets.insert("feature:xsavec".to_string());
        }
        if is_x86_feature_detected!("cmpxchg16b") {
            targets.insert("feature:cmpxchg16b".to_string());
        }
        if is_x86_feature_detected!("adx") {
            targets.insert("feature:adx".to_string());
        }
        if is_x86_feature_detected!("rtm") {
            targets.insert("feature:rtm".to_string());
        }
        if is_x86_feature_detected!("movbe") {
            targets.insert("feature:movbe".to_string());
        }
        if is_x86_feature_detected!("ermsb") {
            targets.insert("feature:ermsb".to_string());
        }
    }

    targets
});
