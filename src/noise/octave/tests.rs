//! Octave 噪声 golden 向量测试（向量来源见 `perlin/tests.rs` 头部注释）。
//!
//! beta 地形的向量由掩码版参考生成（cubiomes 原版越界读，详见
//! `perlin.rs` 模块文档与 `reference/gen/noisevec.c`）。

use super::*;
use crate::rng::JavaRandom;

fn check_octaves(on: &OctaveNoise, expect: &[(u64, u64)], ctx: &str) {
    assert_eq!(on.octaves.len(), expect.len(), "{ctx} octcnt");
    for (i, p) in on.octaves.iter().enumerate() {
        assert_eq!(p.amplitude.to_bits(), expect[i].0, "{ctx} oct {i} amplitude");
        assert_eq!(p.lacunarity.to_bits(), expect[i].1, "{ctx} oct {i} lacunarity");
    }
}

/// (种子, omin, len, [(amplitude, lacunarity)...], [5 个采样值])
/// 采样：sample(12.5,63,-34.25)、sample(-0.25,0,1e5)、
/// sample_amp(100.5,128,-50.25,4,0,false)、sample_amp(...,4,64,false)、
/// sample_amp(100.5,0,-50.25,0,0,true)。
type OctJavaVec = (i64, i32, i32, &'static [(u64, u64)], [u64; 5]);

const OCTAVE_JAVA: &[OctJavaVec] = &[
    (0, -1, 2, &[(0x3fd5555555555555,0x3ff0000000000000),(0x3fe5555555555555,0x3fe0000000000000)], [0xbfadbe3e89a6b474,0x3fd9e6e4c7bc8146,0xbfd1cbf1ad18e82b,0xbfd1cbf1ad18e82b,0xbfd116196625ac08]),
    (0, 0, 1, &[(0x3ff0000000000000,0x3ff0000000000000)], [0xbfe80d91cf801fde,0x3fded3e4f8e88724,0xbfc7b2bc9b279436,0xbfc7b2bc9b279436,0xbfcff716eb7e61e8]),
    (0, -3, 4, &[(0x3fb1111111111111,0x3ff0000000000000),(0x3fc1111111111111,0x3fe0000000000000),(0x3fd1111111111111,0x3fd0000000000000),(0x3fe1111111111111,0x3fc0000000000000)], [0x3fb80ed3c9b94989,0x3fd0e66e1e6b303f,0xbfb2246f047ca60a,0xbf98b05f0ebebca8,0xbf967073fe0e0f1a]),
    (0, -5, 3, &[(0x3fc2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fb0000000000000),(0x3fe2492492492492,0x3fa0000000000000)], [0x3fd38868000710c5,0xbfbaf3ff43c045ca,0x3fc59eeb8c516c88,0x3fc5c8420b22c3d3,0xbf8d5ea1b964cd88]),
    (0, -6, 2, &[(0x3fd5555555555555,0x3fa0000000000000),(0x3fe5555555555555,0x3f90000000000000)], [0xbfc20c3baf901616,0xbfb8544a188d1ee2,0x3fc6cf3abfd528d8,0x3fea68b551964107,0x3fa526c1fbbf1609]),
    (1, -1, 2, &[(0x3fd5555555555555,0x3ff0000000000000),(0x3fe5555555555555,0x3fe0000000000000)], [0x3fb565b1d54482d0,0xbf962e7c7baa516a,0x3fdc61e5969cab30,0x3fdc61e5969cab30,0xbfc60b90d6a133ee]),
    (1, 0, 1, &[(0x3ff0000000000000,0x3ff0000000000000)], [0x3fc41859cde8f23c,0x3fbeacaf93e5b171,0x3fdce8088b2b8d4a,0x3fdce8088b2b8d4a,0xbfbd9dc3c53a9d80]),
    (1, -3, 4, &[(0x3fb1111111111111,0x3ff0000000000000),(0x3fc1111111111111,0x3fe0000000000000),(0x3fd1111111111111,0x3fd0000000000000),(0x3fe1111111111111,0x3fc0000000000000)], [0x3fadd9d8811b40dd,0x3fb275d215c5e418,0x3fa4c400b50ba0b4,0x3fa4c400b50ba0b4,0xbfbfcef43ac0ea88]),
    (1, -5, 3, &[(0x3fc2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fb0000000000000),(0x3fe2492492492492,0x3fa0000000000000)], [0xbfc29633124548da,0x3fc8f41f77e73e67,0xbfb88c4b803eabd0,0xbfe482a958799665,0xbfc2c783310e6626]),
    (1, -6, 2, &[(0x3fd5555555555555,0x3fa0000000000000),(0x3fe5555555555555,0x3f90000000000000)], [0xbfd2177d012b65b7,0xbfb34e649ab058cc,0xbfb64b06619af3d0,0xbfd4d9ecdb4dd195,0x3fb534c202600a66]),
    (-1, -1, 2, &[(0x3fd5555555555555,0x3ff0000000000000),(0x3fe5555555555555,0x3fe0000000000000)], [0x3f65a36496c78400,0xbfb1828656bea964,0xbfab5f738a70d52a,0xbfab5f738a70d52a,0xbfa862f6755958e3]),
    (-1, 0, 1, &[(0x3ff0000000000000,0x3ff0000000000000)], [0x3fc4a54791fcaf45,0x3fdb7aebb30d8b21,0x3fc467facf299efa,0x3fc467facf299efa,0x3f9da2ede691349c]),
    (-1, -3, 4, &[(0x3fb1111111111111,0x3ff0000000000000),(0x3fc1111111111111,0x3fe0000000000000),(0x3fd1111111111111,0x3fd0000000000000),(0x3fe1111111111111,0x3fc0000000000000)], [0x3fcf94aa3199cd9a,0x3fc6243a6219e26f,0x3fdbce08146de75a,0x3fe2a2a46efd6902,0xbfbd020df6313c2f]),
    (-1, -5, 3, &[(0x3fc2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fb0000000000000),(0x3fe2492492492492,0x3fa0000000000000)], [0x3fb7e4aceba09435,0x3fc4fb6c0f3dd80e,0xbf82d750a160f950,0x3f9f25b67cd00358,0x3fb85d81cdfa1117]),
    (-1, -6, 2, &[(0x3fd5555555555555,0x3fa0000000000000),(0x3fe5555555555555,0x3f90000000000000)], [0xbfc198b0f0bf9705,0x3fd4fb815f6ed3f2,0xbfc81fcdea1d998e,0xbfb3f2eec911a24c,0x3fd067149ecbf6e4]),
];

#[test]
fn octave_init_and_sample_java_match_c() {
    for (seed, omin, len, octs, vs) in OCTAVE_JAVA {
        let mut rng = JavaRandom::new(*seed);
        let on = OctaveNoise::new_java(&mut rng, *omin, *len);
        let ctx = format!("seed {seed} omin {omin} len {len}");
        check_octaves(&on, octs, &ctx);
        let got = [
            on.sample(12.5, 63.0, -34.25),
            on.sample(-0.25, 0.0, 1e5),
            on.sample_amp(100.5, 128.0, -50.25, 4.0, 0.0, false),
            on.sample_amp(100.5, 128.0, -50.25, 4.0, 64.0, false),
            on.sample_amp(100.5, 0.0, -50.25, 0.0, 0.0, true),
        ];
        for (i, g) in got.iter().enumerate() {
            assert_eq!(g.to_bits(), vs[i], "{ctx} sample {i}");
        }
    }
}

/// Beta 初始化族（与 cubiomes `initSurfaceNoiseBeta` 相同顺序）：
/// octmin(16, 684.412) -> octmax(16, 684.412) -> octmain(8, 684.412/80)
/// -> skip(262*8) -> octcontA(10, 1.121)。
const OCTAVE_BETA: &[(i64, [u64; 6])] = &[
    (0, [0x40cd8a354d3eb1ac,0x40d72598726d6755,0xc0414154f42bfff4,0x4051eae8e1cf6129,0xc04ef6947f4b6cfb,0xc05f0f82b8bab932]),
    (1, [0xc0520c9f9299f3c0,0xc0dcc7ceea80089d,0x404dbce762e16cdf,0xc0570e743dc8541b,0x403d1f02968ec968,0x4054fe76a41e1597]),
    (-1, [0xc0cd14a02138ec40,0xc0c4a93abcc53afc,0xc041d5731cb20d1d,0x40655eb12b0bba60,0xc03d1608cbedc166,0x402f1497764a33eb]),
];

#[test]
fn octave_beta_matches_c() {
    for (seed, vs) in OCTAVE_BETA {
        let mut rng = JavaRandom::new(*seed);
        let omin = OctaveNoise::new_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let omax = OctaveNoise::new_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let omain = OctaveNoise::new_beta(&mut rng, 8, 684.412 / 80.0, 0.5, 1.0, 2.0);
        rng.skip(262 * 8);
        let oc = OctaveNoise::new_beta(&mut rng, 10, 1.121, 0.5, 1.0, 2.0);
        let got = [
            omin.sample(0.125, 4.0, -8.5),
            omax.sample(7.5, -3.25, 11.0),
            omain.sample(-16.25, 0.5, 32.0),
            oc.sample_amp(3.5, 0.0, -7.25, 0.0, 0.0, true),
            omain.sample_beta17_biome(128.5, -256.75),
            omain.sample_beta17_biome(-0.5, 0.25),
        ];
        for (i, g) in got.iter().enumerate() {
            assert_eq!(g.to_bits(), vs[i], "seed {seed} sample {i}");
        }
    }
}

/// Beta 地形采样（掩码版参考）。初始化顺序：octmin(16) -> octmain(8)。
const OCTAVE_BETA_TERRAIN: &[(i64, [u64; 8])] = &[
    (0, [0x40c60f502ccbe737,0x40bfe72ba1a922da,0xc04bd4c11da89774,0xc048e01e6bd1c629,0xc025b62bdc2b8689,0xc02dd61fa48d9c0c,0x40a86e52b6d99786,0x40b12b179c46bc57]),
    (1, [0xc0aec89d19d796c8,0xc0b918352e7efa58,0x40448b417e0e4346,0x404645926cb98aa3,0x404249921b7bc36e,0x4046c607e0e41d7e,0xc093c4184fa6a866,0xc0a7a045cec5cf4c]),
    (-1, [0xc0c5fd4725c8d028,0xc0bdb8ca545011a5,0x405c296fe100dc17,0x4059d0c4ea455d31,0xc05376af5fed1ef4,0xc052a3815620a7f8,0x40c08cc11106418d,0x40c19107fc85f6e6]),
];

#[test]
fn octave_beta17_terrain_matches_masked_c() {
    for (seed, vs) in OCTAVE_BETA_TERRAIN {
        let mut rng = JavaRandom::new(*seed);
        let omin = OctaveNoise::new_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let omain = OctaveNoise::new_beta(&mut rng, 8, 684.412 / 80.0, 0.5, 1.0, 2.0);
        let mut v = [0.0; 2];
        let mut got = [0u64; 8];
        omin.sample_beta17_terrain(&mut v, 12.5 * 0.25, -34.25 * 0.25, false, 0.0);
        got[0] = v[0].to_bits();
        got[1] = v[1].to_bits();
        omain.sample_beta17_terrain(&mut v, -8.0, 77.5, true, 0.0);
        got[2] = v[0].to_bits();
        got[3] = v[1].to_bits();
        omain.sample_beta17_terrain(&mut v, 1000.25, -999.5, true, 4.0);
        got[4] = v[0].to_bits();
        got[5] = v[1].to_bits();
        omin.sample_beta17_terrain(&mut v, -1e5, 2e5, false, 0.0);
        got[6] = v[0].to_bits();
        got[7] = v[1].to_bits();
        assert_eq!(&got, vs, "seed {seed}");
    }
}

/// Xoroshiro 倍频参数（与 noisevec.c 中 xoct_params 一致）：
/// (omin, amplitudes, nmax)。
const XOCT_PARAMS: &[(i32, &[f64], i32)] = &[
    (-3, &[1.0, 1.0, 1.0], -1),
    (-5, &[1.0, 0.0, 1.0, 0.0, 1.0], -1),
    (-4, &[1.0, 1.0, 1.0, 1.0], 2),
    (-4, &[1.0, 1.0, 1.0, 1.0], 0), // nmax == 0：一个 octave 都不生成
    (-1, &[1.0, 1.0], 1),
    (-12, &[1.0, 1.0, 1.0], -1),
];

/// (种子, 参数下标, C 返回的 n, [(amplitude, lacunarity)...], [2 个采样值])
type OctXorVec = (u64, usize, i32, &'static [(u64, u64)], [u64; 2]);

const OCTAVE_XOR: &[OctXorVec] = &[
    (0, 0, 3, &[(0x3fe2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fd0000000000000),(0x3fc2492492492492,0x3fe0000000000000)], [0xbfd17e03dbea7d4f,0xbfb6914bd9e2b210]),
    (0, 1, 3, &[(0x3fe0842108421084,0x3fa0000000000000),(0x3fc0842108421084,0x3fc0000000000000),(0x3fa0842108421084,0x3fe0000000000000)], [0xbfc28ee9f6af99c5,0xbfabf198b9c670e3]),
    (0, 2, 2, &[(0x3fe1111111111111,0x3fb0000000000000),(0x3fd1111111111111,0x3fc0000000000000)], [0xbfcf55247c5cb6a9,0xbfc3089330484005]),
    (0, 3, 0, &[], [0x0000000000000000,0x0000000000000000]),
    (0, 4, 1, &[(0x3fe5555555555555,0x3fe0000000000000)], [0xbfb4cb7722ac2b45,0x3fcb9447a0b96241]),
    (0, 5, 3, &[(0x3fe2492492492492,0x3f30000000000000),(0x3fd2492492492492,0x3f40000000000000),(0x3fc2492492492492,0x3f50000000000000)], [0x3fb21256002ae512,0xbf94d3a830a37ce2]),
    (1, 0, 3, &[(0x3fe2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fd0000000000000),(0x3fc2492492492492,0x3fe0000000000000)], [0x3f8f77fff406d502,0x3fa8036d8ac7a155]),
    (1, 1, 3, &[(0x3fe0842108421084,0x3fa0000000000000),(0x3fc0842108421084,0x3fc0000000000000),(0x3fa0842108421084,0x3fe0000000000000)], [0xbfb18ccabf37dec5,0x3fc156c4ea010828]),
    (1, 2, 2, &[(0x3fe1111111111111,0x3fb0000000000000),(0x3fd1111111111111,0x3fc0000000000000)], [0x3fd6baaceff950bb,0x3fc9d02bec335059]),
    (1, 3, 0, &[], [0x0000000000000000,0x0000000000000000]),
    (1, 4, 1, &[(0x3fe5555555555555,0x3fe0000000000000)], [0xbfb22ede979e38bc,0x3fbc196757ad5716]),
    (1, 5, 3, &[(0x3fe2492492492492,0x3f30000000000000),(0x3fd2492492492492,0x3f40000000000000),(0x3fc2492492492492,0x3f50000000000000)], [0x3fca346bf7f2111e,0xbfb6b3c37ab20151]),
    (244837814094590, 0, 3, &[(0x3fe2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fd0000000000000),(0x3fc2492492492492,0x3fe0000000000000)], [0x3f938537c40f7d68,0x3fc7a6a2fed36ffb]),
    (244837814094590, 1, 3, &[(0x3fe0842108421084,0x3fa0000000000000),(0x3fc0842108421084,0x3fc0000000000000),(0x3fa0842108421084,0x3fe0000000000000)], [0xbfbaf6d4088b2dc5,0x3fc838cc328d60ac]),
    (244837814094590, 2, 2, &[(0x3fe1111111111111,0x3fb0000000000000),(0x3fd1111111111111,0x3fc0000000000000)], [0xbf7a319b1573c2cc,0x3fc277e323b99511]),
    (244837814094590, 3, 0, &[], [0x0000000000000000,0x0000000000000000]),
    (244837814094590, 4, 1, &[(0x3fe5555555555555,0x3fe0000000000000)], [0xbfc5fe4012d5d9d5,0x3fa5c7eb1a91bf50]),
    (244837814094590, 5, 3, &[(0x3fe2492492492492,0x3f30000000000000),(0x3fd2492492492492,0x3f40000000000000),(0x3fc2492492492492,0x3f50000000000000)], [0x3fce8ff66b678eee,0xbfd2b07b38651f09]),
    ((-12345i64) as u64, 0, 3, &[(0x3fe2492492492492,0x3fc0000000000000),(0x3fd2492492492492,0x3fd0000000000000),(0x3fc2492492492492,0x3fe0000000000000)], [0xbfb29108ede02e69,0xbf4120f9ed9957e0]),
    ((-12345i64) as u64, 1, 3, &[(0x3fe0842108421084,0x3fa0000000000000),(0x3fc0842108421084,0x3fc0000000000000),(0x3fa0842108421084,0x3fe0000000000000)], [0xbfc86c0661c48f52,0x3fd4d5a867ab1780]),
    ((-12345i64) as u64, 2, 2, &[(0x3fe1111111111111,0x3fb0000000000000),(0x3fd1111111111111,0x3fc0000000000000)], [0xbfb160f0afb4cb1e,0xbf507fdf058b5680]),
    ((-12345i64) as u64, 3, 0, &[], [0x0000000000000000,0x0000000000000000]),
    ((-12345i64) as u64, 4, 1, &[(0x3fe5555555555555,0x3fe0000000000000)], [0xbf954bd911aa3c5d,0xbfb46cfcb2fedc2d]),
    ((-12345i64) as u64, 5, 3, &[(0x3fe2492492492492,0x3f30000000000000),(0x3fd2492492492492,0x3f40000000000000),(0x3fc2492492492492,0x3f50000000000000)], [0xbfd1e634d2105692,0xbfc02d189a7cb5eb]),
];

#[test]
fn octave_xoroshiro_matches_c() {
    for (seed, q, n, octs, vs) in OCTAVE_XOR {
        let mut xr = Xoroshiro::new(*seed);
        let (omin, amps, nmax) = XOCT_PARAMS[*q];
        let on = OctaveNoise::new_xoroshiro(&mut xr, amps, omin, nmax);
        let ctx = format!("seed {seed} param {q}");
        assert_eq!(on.octaves.len() as i32, *n, "{ctx} n");
        check_octaves(&on, octs, &ctx);
        let got = [
            on.sample(12.5, 63.0, -34.25),
            on.sample(-0.25, 0.0, 1e5),
        ];
        for (i, g) in got.iter().enumerate() {
            assert_eq!(g.to_bits(), vs[i], "{ctx} sample {i}");
        }
    }
}
