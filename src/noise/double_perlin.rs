//! DoublePerlin 噪声（两组倍频噪声按 337/331 频率比叠加）。
//!
//! 移植 cubiomes `noise.c` 的 `doublePerlinInit` / `xDoublePerlinInit` /
//! `sampleDoublePerlin`。C 结构体里的两个 octave 缓冲区在这里合并进
//! 两个拥有所有权的 [`OctaveNoise`]。

use crate::rng::{JavaRandom, Xoroshiro};

use super::octave::OctaveNoise;

/// `xDoublePerlinInit` 的总振幅表：`(5/3) * len / (len + 1)`，
/// 下标为去除首尾零振幅后的 `len`（0..=9）。
const AMP_INI: [f64; 10] = [
    0.0,
    5.0 / 6.0,
    10.0 / 9.0,
    15.0 / 12.0,
    20.0 / 15.0,
    25.0 / 18.0,
    30.0 / 21.0,
    35.0 / 24.0,
    40.0 / 27.0,
    45.0 / 30.0,
];

/// DoublePerlin 噪声（对应 cubiomes `DoublePerlinNoise`）。
#[derive(Clone, Debug)]
pub struct DoublePerlinNoise {
    /// 总振幅（对两组 octave 之和的缩放）。
    pub amplitude: f64,
    pub oct_a: OctaveNoise,
    pub oct_b: OctaveNoise,
}

impl DoublePerlinNoise {
    /// `doublePerlinInit`：旧版（≤1.17）初始化。
    ///
    /// 要求 `len >= 1 && omin + len <= 0`。注意 C 里 `octA`/`octB` 共用同一
    /// 个种子状态**顺序**初始化（B 接续 A 消耗后的状态）。
    pub fn new_java(rng: &mut JavaRandom, omin: i32, len: i32) -> Self {
        // C: (10.0 / 6.0) * len / (len + 1)，注意是 * len 后再 / (len+1)。
        let amplitude = (10.0 / 6.0) * len as f64 / (len + 1) as f64;
        let oct_a = OctaveNoise::new_java(rng, omin, len);
        let oct_b = OctaveNoise::new_java(rng, omin, len);
        DoublePerlinNoise {
            amplitude,
            oct_a,
            oct_b,
        }
    }

    /// `xDoublePerlinInit`：1.18+ 初始化。
    ///
    /// `nmax > 0` 时限制两组 octave 的总数：A 组 `ceil(nmax/2)` 个、
    /// B 组 `floor(nmax/2)` 个；`nmax <= 0` 表示不限制。
    ///
    /// 总振幅下标用的 `len` 会先裁掉 `amplitudes` 首尾连续的零
    /// （C 的两个 `for` 循环），故前置条件与 C 相同：至少一个非零振幅。
    pub fn new_xoroshiro(
        xr: &mut Xoroshiro,
        amplitudes: &[f64],
        omin: i32,
        nmax: i32,
    ) -> Self {
        let mut na = -1;
        let mut nb = -1;
        if nmax > 0 {
            na = (nmax + 1) >> 1;
            nb = nmax - na;
        }
        let oct_a = OctaveNoise::new_xoroshiro(xr, amplitudes, omin, na);
        let oct_b = OctaveNoise::new_xoroshiro(xr, amplitudes, omin, nb);

        // 与 C 相同的双向裁剪：先去尾部零，再去头部零。
        let mut len = amplitudes.len();
        let mut i = len as i32 - 1;
        while i >= 0 && amplitudes[i as usize] == 0.0 {
            len -= 1;
            i -= 1;
        }
        let mut i = 0usize;
        while amplitudes[i] == 0.0 {
            len -= 1;
            i += 1;
        }
        let amplitude = AMP_INI[len];
        DoublePerlinNoise {
            amplitude,
            oct_a,
            oct_b,
        }
    }

    /// `sampleDoublePerlin`：B 组采样频率固定乘以 `337/331`。
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        const F: f64 = 337.0 / 331.0;
        let mut v = 0.0;
        v += self.oct_a.sample(x, y, z);
        v += self.oct_b.sample(x * F, y * F, z * F);
        v * self.amplitude
    }
}

#[cfg(test)]
mod tests;
