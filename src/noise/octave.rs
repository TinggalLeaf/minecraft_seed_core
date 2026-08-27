//! Perlin 倍频（octave）噪声。
//!
//! 移植 cubiomes `noise.c` 的 `octaveInit` / `octaveInitBeta` /
//! `xOctaveInit` / `sampleOctave` / `sampleOctaveAmp` /
//! `sampleOctaveBeta17Biome` / `sampleOctaveBeta17Terrain`。
//!
//! C 的 `OctaveNoise { int octcnt; PerlinNoise *octaves; }` 用调用方提供的
//! 缓冲区；这里改用拥有所有权的 `Vec<PerlinNoise>`，`octaves.len()` 即
//! `octcnt`。
//!
//! 注意：`noise.h` 中声明的 `sampleOctave2D` 在 cubiomes 源码里**没有任何
//! 定义**（残留声明），故不移植。
//!
//! 本版 cubiomes 的 `maintainPrecision(x)` 是恒等函数（`noise.h` 注释说明：
//! cubiomes 全程用 double，float 误差修正没有意义），移植时直接省略，
//! 保留注释以便对照。

use crate::rng::{JavaRandom, Xoroshiro};

use super::perlin::PerlinNoise;

/// `xOctaveInit` 的每-octave 种子异或盐：`md5("octave_-12")` …
/// `md5("octave_0")` 各取前 8 字节作为 lo/hi。
const MD5_OCTAVE_N: [[u64; 2]; 13] = [
    [0xb198de63a8012672, 0x7b84cad43ef7b5a8], // md5 "octave_-12"
    [0x0fd787bfbc403ec3, 0x74a4a31ca21b48b8], // md5 "octave_-11"
    [0x36d326eed40efeb2, 0x5be9ce18223c636a], // md5 "octave_-10"
    [0x082fe255f8be6631, 0x4e96119e22dedc81], // md5 "octave_-9"
    [0x0ef68ec68504005e, 0x48b6bf93a2789640], // md5 "octave_-8"
    [0xf11268128982754f, 0x257a1d670430b0aa], // md5 "octave_-7"
    [0xe51c98ce7d1de664, 0x5f9478a733040c45], // md5 "octave_-6"
    [0x6d7b49e7e429850a, 0x2e3063c622a24777], // md5 "octave_-5"
    [0xbd90d5377ba1b762, 0xc07317d419a7548d], // md5 "octave_-4"
    [0x53d39c6752dac858, 0xbcd1c5a80ab65b3e], // md5 "octave_-3"
    [0xb4a24d7a84e7677b, 0x023ff9668e89b5c4], // md5 "octave_-2"
    [0xdffa22b534c5f608, 0xb9b67517d3665ca9], // md5 "octave_-1"
    [0xd50708086cef4d7c, 0x6e1651ecc7f43309], // md5 "octave_0"
];

/// `xOctaveInit` 的初始频隙表，下标为 `-omin`（0..=12）。
const LACUNA_INI: [f64; 13] = [
    1.0,
    0.5,
    0.25,
    1.0 / 8.0,
    1.0 / 16.0,
    1.0 / 32.0,
    1.0 / 64.0,
    1.0 / 128.0,
    1.0 / 256.0,
    1.0 / 512.0,
    1.0 / 1024.0,
    1.0 / 2048.0,
    1.0 / 4096.0,
];

/// `xOctaveInit` 的初始持久度表，下标为 `len`（0..=9）。
/// 即 `2^len / (2^(len+1) - 1)`；常量表达式与 C 的 `2./3` 等逐位一致。
const PERSIST_INI: [f64; 10] = [
    0.0,
    1.0,
    2.0 / 3.0,
    4.0 / 7.0,
    8.0 / 15.0,
    16.0 / 31.0,
    32.0 / 63.0,
    64.0 / 127.0,
    128.0 / 255.0,
    256.0 / 511.0,
];

/// 倍频噪声（对应 cubiomes `OctaveNoise`）。
#[derive(Clone, Debug)]
pub struct OctaveNoise {
    /// 各 octave；`len()` 即 C 的 `octcnt`。
    pub octaves: Vec<PerlinNoise>,
}

impl OctaveNoise {
    /// `octaveInit`：旧版（≤1.17）倍频初始化。
    ///
    /// 要求 `len >= 1 && omin + len - 1 <= 0`（C 中越界时只打印错误并留下
    /// 未初始化对象，这里改为 panic）。
    ///
    /// 反直觉细节：当 `end == 0`（即最高 octave 频率为 1）时，第 0 个
    /// octave 直接用当前随机状态初始化；否则先 `skipNextN(seed, -end*262)`
    /// 跳过等效于初始化 `-end` 个 Perlin 的随机数（每个 Perlin 消耗
    /// 3 + 256 + 若干次拒绝采样 = 262 次 `next`），使种子对齐到最低频
    /// octave——两次初始化等价于"从低频到高频"还是"从高频到低频"取决于
    /// MC 版本，该 skip 保证了与 MC 的种子消费顺序一致。
    pub fn new_java(rng: &mut JavaRandom, omin: i32, len: i32) -> Self {
        let end = omin + len - 1;
        assert!(
            len >= 1 && end <= 0,
            "octaveInit: unsupported octave range (omin={omin}, len={len})"
        );
        let mut persist = 1.0 / (((1i64 << len) as f64) - 1.0);
        let mut lacuna = 2.0f64.powi(end);

        let mut octaves = Vec::with_capacity(len as usize);
        let mut i = 0;
        if end == 0 {
            let mut p = PerlinNoise::new_java(rng);
            p.amplitude = persist;
            p.lacunarity = lacuna;
            octaves.push(p);
            persist *= 2.0;
            lacuna *= 0.5;
            i = 1;
        } else {
            // C: skipNextN(seed, -end*262)，int 乘法结果为正。
            rng.skip((-end * 262) as u64);
        }

        for _ in i..len {
            let mut p = PerlinNoise::new_java(rng);
            p.amplitude = persist;
            p.lacunarity = lacuna;
            octaves.push(p);
            persist *= 2.0;
            lacuna *= 0.5;
        }

        OctaveNoise { octaves }
    }

    /// `octaveInitBeta`：Beta 1.7 风格初始化（显式给出初始频隙/持久度及
    /// 各自的倍率）。
    pub fn new_beta(
        rng: &mut JavaRandom,
        octcnt: i32,
        lac: f64,
        lac_mul: f64,
        persist: f64,
        persist_mul: f64,
    ) -> Self {
        let mut persist = persist;
        let mut lac = lac;
        let mut octaves = Vec::with_capacity(octcnt.max(0) as usize);
        for _ in 0..octcnt {
            let mut p = PerlinNoise::new_java(rng);
            p.amplitude = persist;
            p.lacunarity = lac;
            octaves.push(p);
            persist *= persist_mul;
            lac *= lac_mul;
        }
        OctaveNoise { octaves }
    }

    /// `xOctaveInit`：1.18+ 倍频初始化。
    ///
    /// 每个非零振幅的 octave 用 `(xlo, xhi) ^ md5("octave_{omin+i}")` 作为
    /// Xoroshiro 状态独立派生；振幅为 0 的 octave 被跳过（不占用返回的
    /// `octaves`，但**仍消耗** lacuna/persist 的倍率推进）。
    ///
    /// `nmax <= 0` 表示不限制 octave 数（C 用 `n != nmax` 判断，`nmax`
    /// 为负时恒真；注意 `nmax == 0` 时 C 会一个都不生成，此行为原样保留）。
    ///
    /// 返回值就是 `Self`（C 返回的 `n` 等于 `octaves.len()`）。
    pub fn new_xoroshiro(
        xr: &mut Xoroshiro,
        amplitudes: &[f64],
        omin: i32,
        nmax: i32,
    ) -> Self {
        let len = amplitudes.len() as i32;
        assert!((0..=12).contains(&-omin), "xOctaveInit: omin out of range (-12..=0)");
        assert!(len < 10, "xOctaveInit: len out of range (0..=9)");
        // md5 盐表下标为 12 + omin + i（i < len），C 只（在 DEBUG 下）检查
        // lacuna/persist 表，不检查这一项；越界在 C 里是静默 UB，
        // 这里显式断言。约束等价于 omin + len <= 1。
        assert!(
            12 + omin + (len - 1) <= 12,
            "xOctaveInit: md5 salt index out of range (omin + len must be <= 1)"
        );
        let mut lacuna = LACUNA_INI[(-omin) as usize];
        let mut persist = PERSIST_INI[len as usize];
        let xlo = xr.next_long();
        let xhi = xr.next_long();

        let mut octaves = Vec::new();
        let mut i = 0;
        let mut n = 0;
        while i < len && n != nmax {
            if amplitudes[i as usize] != 0.0 {
                let mut pxr = Xoroshiro::from_state(
                    xlo ^ MD5_OCTAVE_N[(12 + omin + i) as usize][0],
                    xhi ^ MD5_OCTAVE_N[(12 + omin + i) as usize][1],
                );
                let mut p = PerlinNoise::new_xoroshiro(&mut pxr);
                p.amplitude = amplitudes[i as usize] * persist;
                p.lacunarity = lacuna;
                octaves.push(p);
                n += 1;
            }
            lacuna *= 2.0;
            persist *= 0.5;
            i += 1;
        }

        OctaveNoise { octaves }
    }

    /// `sampleOctave`：3D 倍频采样（无 y 截断）。
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut v = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            // maintainPrecision 在 cubiomes 中是恒等函数，略。
            let ax = x * lf;
            let ay = y * lf;
            let az = z * lf;
            let pv = p.sample(ax, ay, az, 0.0, 0.0);
            v += p.amplitude * pv;
        }
        v
    }

    /// `sampleOctaveAmp`：带 y 截断的 3D 倍频采样。
    ///
    /// `ydefault` 为真时传 `-p.b` 作为 y：进入 `samplePerlin` 后
    /// `y + b == 0`，命中 `y == 0.0` 的缓存快路径（避免重复计算
    /// fade/置换下标）。这是 cubiomes 对"此 octave 的 y 不相关"的惯例
    /// 表达。
    pub fn sample_amp(
        &self,
        x: f64,
        y: f64,
        z: f64,
        yamp: f64,
        ymin: f64,
        ydefault: bool,
    ) -> f64 {
        let mut v = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            let ax = x * lf;
            let ay = if ydefault { -p.b } else { y * lf };
            let az = z * lf;
            let pv = p.sample(ax, ay, az, yamp * lf, ymin * lf);
            v += p.amplitude * pv;
        }
        v
    }

    /// `sampleOctaveBeta17Biome`：Beta 1.7 群系（温度/湿度）采样。
    ///
    /// 注意 z 方向偏移用的是 `p.b` 而非 `p.c`——simplex 是 2D 的，
    /// 只消费两个坐标轴，cubiomes 复用了 b 字段，原样保留。
    pub fn sample_beta17_biome(&self, x: f64, z: f64) -> f64 {
        let mut v = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            let ax = x * lf + p.a;
            let az = z * lf + p.b;
            let pv = p.sample_simplex_2d(ax, az);
            v += p.amplitude * pv;
        }
        v
    }

    /// `sampleOctaveBeta17Terrain`：Beta 1.7 地形垂直剖面采样。
    ///
    /// `v` 会被清零后累加。`y_lac_flag` 为真时 y 方向频隙减半（0.5），
    /// `lacmin != 0.0` 时跳过 `lacunarity > lacmin` 的 octave。
    ///
    /// 底层 `sample_beta17_terrain` 对 C 的越界读做了 `& 0xff` 修正，
    /// 见 [`crate::noise::perlin`] 模块文档。
    pub fn sample_beta17_terrain(
        &self,
        v: &mut [f64; 2],
        x: f64,
        z: f64,
        y_lac_flag: bool,
        lacmin: f64,
    ) {
        v[0] = 0.0;
        v[1] = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            if lacmin != 0.0 && lf > lacmin {
                continue;
            }
            let ax = x * lf;
            let az = z * lf;
            p.sample_beta17_terrain(v, ax, az, if y_lac_flag { 0.5 } else { 1.0 });
        }
    }
}

#[cfg(test)]
mod tests;
