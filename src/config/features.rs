use hashbrown::HashSet;

pub fn get_features() -> HashSet<&'static str> {
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
