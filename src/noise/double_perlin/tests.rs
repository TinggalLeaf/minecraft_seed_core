//! DoublePerlin 噪声 golden 向量测试（向量来源见 `perlin/tests.rs` 头部注释）。

use super::*;
use crate::rng::JavaRandom;

/// 采样点（与 noisevec.c 一致）。
const DP_PTS: &[(f64, f64, f64)] = &[
    (0.5, 63.0, -0.25),
    (-1234.5, 0.0, 5678.75),
    (29999984.0, -64.0, -29999984.0),
];

/// (种子, omin, len, amplitude, [3 个采样值])
const DP_JAVA: &[(i64, i32, i32, u64, [u64; 3])] = &[
    (0, -3, 2, 0x3ff1c71c71c71c72, [0x3fcf90bae84de5b9,0xbfdd97eac6ba96ec,0xbfcba7dda8cb72f1]),
    (0, -1, 1, 0x3feaaaaaaaaaaaab, [0x3fc5089035528495,0x3fad53ab9a8e863e,0xbfd724cbaa422903]),
    (0, -2, 3, 0x3ff4000000000000, [0x3f85e2324826bb50,0x3fe2b41bda9ccd9f,0x3faadb43aeb30956]),
    (1, -3, 2, 0x3ff1c71c71c71c72, [0xbfa8ca24c92ec967,0xbf7a8a300b909ce4,0x3fd835b7a9ffe406]),
    (1, -1, 1, 0x3feaaaaaaaaaaaab, [0x3fc2c02981e9ff95,0xbfaeb0f997e6664c,0xbfd159d39be42654]),
    (1, -2, 3, 0x3ff4000000000000, [0x3fd0d1bd9c28ddbb,0x3fd0a4177b2ab381,0xbfbb31fbba1a2f1d]),
    (-1, -3, 2, 0x3ff1c71c71c71c72, [0x3f8229e895cf7659,0xbf9a76563d180d01,0xbfdcdeb4e0b0791b]),
    (-1, -1, 1, 0x3feaaaaaaaaaaaab, [0x3fc92cf7bf8e6676,0x3fc20ef9b2a20478,0x3fd94696c13f60f2]),
    (-1, -2, 3, 0x3ff4000000000000, [0x3fd2de5106d35e61,0xbfd1b48884871d55,0x3f74b275ecf4a5b4]),
];

#[test]
fn double_perlin_java_matches_c() {
    for (seed, omin, len, amp, vs) in DP_JAVA {
        let mut rng = JavaRandom::new(*seed);
        let dp = DoublePerlinNoise::new_java(&mut rng, *omin, *len);
        let ctx = format!("seed {seed} omin {omin} len {len}");
        assert_eq!(dp.amplitude.to_bits(), *amp, "{ctx} amplitude");
        assert_eq!(dp.oct_a.octaves.len(), *len as usize, "{ctx} octA len");
        assert_eq!(dp.oct_b.octaves.len(), *len as usize, "{ctx} octB len");
        for (i, &(x, y, z)) in DP_PTS.iter().enumerate() {
            assert_eq!(dp.sample(x, y, z).to_bits(), vs[i], "{ctx} sample {i}");
        }
    }
}

/// Xoroshiro 参数（与 noisevec.c 中 xdp_params 一致）：(omin, amplitudes, nmax)。
const XDP_PARAMS: &[(i32, &[f64], i32)] = &[
    (-4, &[1.0, 1.0, 1.0, 1.0], -1),
    (-4, &[1.0, 0.0, 1.0, 1.0], -1), // 首尾裁剪：len 4 -> 3 -> 2
    (-9, &[1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0], -1),
    (-3, &[1.0, 1.0, 1.0], 4), // nmax 限制：A 组 2 个、B 组 2 个
];

/// (种子, 参数下标, C 返回的 n（两组 octave 总数）, amplitude, [3 个采样值])
const DP_XOR: &[(u64, usize, i32, u64, [u64; 3])] = &[
    (0, 0, 8, 0x3ff5555555555555, [0x3fdb2206327c0e25,0x3fc14f468ca76322,0x3fe45b4f57ab457a]),
    (0, 1, 6, 0x3ff5555555555555, [0x3fd9641aa767816a,0x3fc66aff1140f4ea,0x3fdcfc7686ec1e8d]),
    (0, 2, 18, 0x3ff8000000000000, [0x3fbee6a264314adb,0xbfbf0449a30010dc,0x3fc06ba426840e4c]),
    (0, 3, 4, 0x3ff4000000000000, [0x3fb2d23cd8b710be,0xbfa9c6617b63d352,0x3fe11f17958ac1b7]),
    (1, 0, 8, 0x3ff5555555555555, [0xbfdad27935a22740,0xbfd45e755a20f996,0xbfd875003a7f4d15]),
    (1, 1, 6, 0x3ff5555555555555, [0xbfdb28bded8ed61d,0xbfd28195e772ca7e,0xbfd1246cfe8c629d]),
    (1, 2, 18, 0x3ff8000000000000, [0x3fd208f2bc7b0150,0xbfc2b319ba9b92a4,0x3fee09536dcfe439]),
    (1, 3, 4, 0x3ff4000000000000, [0x3fb4fb3f9dee3585,0x3f72f43d305c8748,0xbfdff852b8a81d9a]),
    (244837814094590, 0, 8, 0x3ff5555555555555, [0x3f87e95ca96c9d7a,0xbfdb642569155bcd,0x3fb399f17f3873ad]),
    (244837814094590, 1, 6, 0x3ff5555555555555, [0x3fd0540457709a10,0xbfd68607cfd2f290,0xbf979cfbf965f105]),
    (244837814094590, 2, 18, 0x3ff8000000000000, [0x3fde7e555469fb90,0x3fde3fef60f47022,0xbf9c63a307b9d48f]),
    (244837814094590, 3, 4, 0x3ff4000000000000, [0xbfe92ee86a8b011f,0xbfc6f79bb40172e7,0x3fc27484e2c6fec9]),
    ((-12345i64) as u64, 0, 8, 0x3ff5555555555555, [0x3fd468ae48ebce49,0x3faef39077674fac,0x3fb4cce774cc1d82]),
    ((-12345i64) as u64, 1, 6, 0x3ff5555555555555, [0x3fd9513ffbcf69b0,0x3fc4f4c59b6545ad,0x3fab08ce6cc6d370]),
    ((-12345i64) as u64, 2, 18, 0x3ff8000000000000, [0x3fd78395ecb5d9bf,0xbfe2782ec0e6d464,0x3fcad35223030a29]),
    ((-12345i64) as u64, 3, 4, 0x3ff4000000000000, [0xbfc2fd288a2a76a8,0xbfdab86bbb662c5c,0xbfb1764ca0f37166]),
];

#[test]
fn double_perlin_xoroshiro_matches_c() {
    for (seed, q, n, amp, vs) in DP_XOR {
        let mut xr = Xoroshiro::new(*seed);
        let (omin, amps, nmax) = XDP_PARAMS[*q];
        let dp = DoublePerlinNoise::new_xoroshiro(&mut xr, amps, omin, nmax);
        let ctx = format!("seed {seed} param {q}");
        let total = dp.oct_a.octaves.len() + dp.oct_b.octaves.len();
        assert_eq!(total as i32, *n, "{ctx} n");
        assert_eq!(dp.amplitude.to_bits(), *amp, "{ctx} amplitude");
        for (i, &(x, y, z)) in DP_PTS.iter().enumerate() {
            assert_eq!(dp.sample(x, y, z).to_bits(), vs[i], "{ctx} sample {i}");
        }
    }
}
