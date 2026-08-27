//! 由 `reference/gen/biomevec.c`（cubiomes C 参考实现）生成的 golden 测试。
//! 数据由 `reference/gen/gen_tests.py` 转换，请勿手工编辑。
//!
//! 所有 f64/f32 按位模式比较（与 C 逐位一致）。

use super::*;
use crate::version::McVersion;

const PTS: [(i32, i32); 4] = [(0, 0), (100, -200), (-5000, 8000), (7499996, -7499996)];

/// setBiomeSeed 后各 climate 噪声的 sampleDoublePerlin 输出（位模式）。
/// 条目：(seed, climate 下标, [4 个采样点的 f64 位模式])。
/// climate 下标顺序即 NP_*：0=temperature 1=humidity 2=continentalness
/// 3=erosion 4=shift 5=weirdness。
const CLIMATE_CASES: &[(i64, usize, [u64; 4])] = &[
    (0, 0, [0x3fbf513532e681eb,0x3fce7fa3fd5459e0,0xbfb2373c80ef436c,0x3fb8482ca271d130,]),
    (0, 1, [0xbf82650dd3053d4e,0x3fa46b9e913fd303,0xbfbcdd2a4fa794d1,0xbf9d3228f82b530d,]),
    (0, 2, [0xbf9bb72286ca68ee,0x3f901a536cb6a0e5,0x3fdd9511ac5d9dc7,0x3fc7e170dc92f42a,]),
    (0, 3, [0xbfbafb4bee33bd28,0x3fd7ad4651cdf239,0xbfc280262e4fd069,0x3fbb7c7f960ec165,]),
    (0, 4, [0xbfe1650f2efc9ae5,0x3fba0dc0fbfaad6e,0xbfc65cf1dc7e5f40,0xbfd451f0884de96c,]),
    (0, 5, [0x3f97bfa8b6ea90ee,0x3fd7e21ec6c451ae,0xbfe19aee774d508b,0xbfb82e1ecb575706,]),
    (1, 0, [0xbfa935bbae774a62,0xbfcff1b640fe3ce2,0xbfe21a0279c1799f,0xbfde6a1606c7a2ff,]),
    (1, 1, [0xbfd295632a6bda31,0xbfd17bcd297822a0,0x3fc050181fa73e90,0x3fc389fddd51eb37,]),
    (1, 2, [0xbfdf602a91d1f080,0xbfd3a5b7dbc3550a,0x3fc3639049852fc1,0x3fd934424e8e41bb,]),
    (1, 3, [0xbfcd4b2cdc41231d,0x3fd3bc08754a2ba1,0x3f8fcd191c190f76,0x3fc011fbe473aab0,]),
    (1, 4, [0x3fd2cb26a92a9081,0xbfe0230eb14a7f53,0xbfc8e152aa6e2f16,0x3fc3f25d44072176,]),
    (1, 5, [0x3fcf3c90a2c44080,0xbfb4c6576cba0a72,0x3fe6a519e6c7abb8,0x3f9515e4c73e1fc6,]),
    (-1, 0, [0x3fc4a1c9214abea0,0xbf99eb722dc7730a,0xbfc973b86eb69002,0xbfc2b7397ace65f6,]),
    (-1, 1, [0xbfc20ba11a8b1fb2,0xbfcf03fa3f2d4336,0x3fd3a6c8685755dc,0xbfbf2024db68163d,]),
    (-1, 2, [0xbfe393ff178294de,0xbfdbafef8d4e8e36,0x3f785f03f35710d8,0x3fcb27b872cfe4ae,]),
    (-1, 3, [0x3fd0226b3553a2ed,0x3fddd3819d910e91,0xbfd21f812ae3d35a,0x3fbde1b9dd38a042,]),
    (-1, 4, [0x3fd2d84fa3f6c94e,0x3fd7f3db0bccca99,0x3fd9f26356e1e83a,0xbfda8fe5fc22871b,]),
    (-1, 5, [0xbfb1552403e9dba2,0xbfb4c366de5222ec,0xbfdfeda4f3e206ce,0x3fe06cf3a486710a,]),
    (12345, 0, [0xbf9954002170a916,0xbfd212b89a4f8bef,0xbfdeb687a18083ba,0xbfdce19050734725,]),
    (12345, 1, [0x3fe5c4964a763b32,0xbfbbe10fe93e6d3c,0x3fd2b94b182ce13b,0xbfc71b7b67c2e7ad,]),
    (12345, 2, [0xbfd9c475c7998ee6,0x3fcc16b7555ad1c6,0xbf41b3eb4bbfc200,0x3fc2b2b783b45ed9,]),
    (12345, 3, [0xbfe7819597748715,0xbfb823e5c5025ef0,0x3fc75a9982a6083b,0x3fdffc01da523c37,]),
    (12345, 4, [0xbfe1305e4effdeba,0x3fe06f514ed9fe4d,0x3fdd2f6e5944e3a8,0xbf9ffbb9322207ac,]),
    (12345, 5, [0x3fc674e1e14ff22d,0xbfd452aa84a91844,0xbfa6c6bf4c2da6b4,0xbfda589fa8d9f8d8,]),
    (-1461259574, 0, [0xbfa0c90c76c11a45,0xbf855bedebb2ed4c,0xbfb257e60aee7fca,0x3fd508f5ee1b473e,]),
    (-1461259574, 1, [0x3fd8f32331eb360b,0xbfb74d343ed39537,0x3fda16d33fdd0fb5,0x3f9a7bdf4d05f944,]),
    (-1461259574, 2, [0xbfd3d04b28881d7c,0x3fd9c4561b1c4a53,0x3fc9469aeebe10ce,0x3fe87aab9d26a2e6,]),
    (-1461259574, 3, [0x3fe6c818fc00fe1e,0x3fc48ad491302ffb,0xbfc265298a5bb11c,0xbfd7e94935078165,]),
    (-1461259574, 4, [0x3fe13a74407c8e14,0xbfd5bfa2ebac6c10,0xbfa15b3ae8d4993c,0x3fbf280260ed9cbe,]),
    (-1461259574, 5, [0x3fbd95ed838d25dd,0xbfa2c8fcfddc1e43,0x3fe28f259eb65372,0x3fa0de8a75798c6f,]),
];

/// large biomes（种子 12345）：(climate 下标, [4 点 f64 位模式])。
const CLIMATE_LARGE_CASES: &[(usize, [u64; 4])] = &[
    (0, [0xbfeac5218b0f8944,0xbfeef450bb269b04,0x3fe9366dedfa1eac,0x3fe24d6adda338a7,]),
    (1, [0x3fc641979d410535,0x3fc137733b35f766,0xbfc75e1d44b84004,0x3fbbc875830ac296,]),
    (2, [0xbfe0aea1318e9f9c,0xbf943d6b6fc123d3,0x3fcde8f3a9a5f288,0xbfbe2930ba837cee,]),
    (3, [0x3fb09a0a17e4f6b9,0xbfabf3b80e0f131c,0xbfbb370530294855,0x3fd8d12544191fdd,]),
    (4, [0xbfe1305e4effdeba,0x3fe06f514ed9fe4d,0x3fdd2f6e5944e3a8,0xbf9ffbb9322207ac,]),
    (5, [0x3fc674e1e14ff22d,0xbfd452aa84a91844,0xbfa6c6bf4c2da6b4,0xbfda589fa8d9f8d8,]),
];

/// getSpline 求值：(输入下标, f32 位模式)。
const SPLINE_CASES: &[(usize, u32)] = &[
    (0, 0x3bf46391),
    (1, 0xbe5dbc58),
    (2, 0x3ee62c49),
    (3, 0x3d343958),
    (4, 0x3ec75f8e),
    (5, 0x00000000),
    (6, 0x3f9d70a4),
];

const SPLINE_VALS: [[f32; 4]; 7] = [
    [0.0, 0.0, 0.0, 0.0],
    [-0.5, 0.3, 0.2, -0.4],
    [0.7, -0.6, -0.1, 0.9],
    [-1.1, 1.0, 0.5, -1.0],
    [1.0, -1.0, -0.8, 0.5],
    [-0.16, 0.0, 0.0, 0.0],
    [0.25, -0.85, 1.0, -0.65],
];

#[test]
fn climate_samples_match_c() {
    for &(seed, c, expect) in CLIMATE_CASES {
        let mut bn = BiomeNoise::new(McVersion::V1_18);
        bn.set_biome_seed(seed as u64, false);
        for (p, &e) in PTS.iter().zip(expect.iter()) {
            let v = bn.climate[c].sample(p.0 as f64, 0.0, p.1 as f64);
            assert_eq!(v.to_bits(), e, "seed {seed} climate {c} pt {p:?}");
        }
    }
}

#[test]
fn climate_samples_large_biomes_match_c() {
    let mut bn = BiomeNoise::new(McVersion::V1_18);
    bn.set_biome_seed(12345, true);
    for &(c, expect) in CLIMATE_LARGE_CASES {
        for (p, &e) in PTS.iter().zip(expect.iter()) {
            let v = bn.climate[c].sample(p.0 as f64, 0.0, p.1 as f64);
            assert_eq!(v.to_bits(), e, "large climate {c} pt {p:?}");
        }
    }
}

#[test]
fn spline_eval_matches_c() {
    let bn = BiomeNoise::new(McVersion::V1_18);
    for &(i, e) in SPLINE_CASES {
        let v = bn.eval_spline(&SPLINE_VALS[i]);
        assert_eq!(v.to_bits(), e, "spline case {i}");
    }
}

