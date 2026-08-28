//! 地表（地形密度）噪声：移植 cubiomes `biomenoise.c` 的 `SurfaceNoise`
//! 结构与 `initSurfaceNoise` / `sampleSurfaceNoise` /
//! `sampleSurfaceNoiseBetween`。
//!
//! 这是 1.18 前主世界与全版本末地的三维地形密度噪声（min/max/main 三组
//! Perlin 倍频的钳制插值），用于：
//!
//! - [`crate::generator::Generator::map_approx_height`] 的地表高度近似
//!   （1.7–1.17 主世界与末地）；
//! - 末地城地形可行性（`crate::structure::viability`）与末地折跃门定位等
//!   finders.c 功能。
//!
//! 注意 `sampleSurfaceNoise` 系列**不使用** `OctaveNoise` 上由
//! `octaveInit` 写入的 amplitude/lacunarity 字段，而是按下标自行推进
//! persist/contrib（C 原样如此）；这些字段仅被 `octdepth` 经
//! [`OctaveNoise::sample_amp`] 的旧版地表偏移采样使用。
//!
//! Beta 1.7 及更早的 `initSurfaceNoiseBeta` / `approxSurfaceBeta` 在
//! [`crate::noise::beta`] 中（`SurfaceNoiseBeta` / `approx_surface_beta`）。

use crate::rng::JavaRandom;
use crate::version::Dimension;

use super::octave::OctaveNoise;

/// `rng.h` 的 `clampedLerp`：`part <= 0` 取 `from`，`part >= 1` 取 `to`，
/// 否则线性插值。
#[inline(always)]
fn clamped_lerp(part: f64, from: f64, to: f64) -> f64 {
    if part <= 0.0 {
        return from;
    }
    if part >= 1.0 {
        return to;
    }
    from + part * (to - from)
}

/// 地表噪声（对应 cubiomes `SurfaceNoise`）。
///
/// C 结构体里的 `oct[16+16+8+4+16]` 缓冲区被拆进五个拥有所有权的
/// [`OctaveNoise`]；`octsurf` 当前没有任何采样函数使用，但初始化它消耗的
/// 随机数影响后续 `octdepth` 的种子对齐，必须保留。
#[derive(Clone, Debug)]
pub struct SurfaceNoise {
    /// 水平/垂直缩放（`xzScale`/`yScale`）。
    pub xz_scale: f64,
    pub y_scale: f64,
    /// 主噪声的步进因子（`xzFactor`/`yFactor`）。
    pub xz_factor: f64,
    pub y_factor: f64,
    /// min/max 密度倍频（各 16 octave，`-15..=0`）。
    pub octmin: OctaveNoise,
    pub octmax: OctaveNoise,
    /// main 密度倍频（8 octave，`-7..=0`）。
    pub octmain: OctaveNoise,
    /// 表层装饰倍频（4 octave，仅主世界初始化；C 移植范围内无采样使用）。
    pub octsurf: OctaveNoise,
    /// 深度/偏移倍频（16 octave，仅主世界初始化；旧版地表高度近似使用）。
    pub octdepth: OctaveNoise,
}

impl SurfaceNoise {
    /// `initSurfaceNoise`：按维度与种子初始化。
    ///
    /// 随机流顺序（Java LCG）：min(16) → max(16) → main(8) → 主世界再
    /// surf(4) → 跳过 262×10 次 `next`（对齐 MC 中被省略的噪声）→
    /// depth(16)。末地分支只初始化前三组并使用不同的缩放常量。
    ///
    /// C 的 `dim` 参数只有 `DIM_END` 与其它（按主世界处理）两种语义；
    /// 传 [`Dimension::Nether`] 按 C 行为走主世界分支（调用方不应这么做）。
    pub fn new(dim: Dimension, seed: u64) -> Self {
        let mut rng = JavaRandom::new(seed as i64);
        let octmin = OctaveNoise::new_java(&mut rng, -15, 16);
        let octmax = OctaveNoise::new_java(&mut rng, -15, 16);
        let octmain = OctaveNoise::new_java(&mut rng, -7, 8);
        let mut sn = SurfaceNoise {
            octmin,
            octmax,
            octmain,
            octsurf: OctaveNoise { octaves: Vec::new() },
            octdepth: OctaveNoise { octaves: Vec::new() },
            xz_scale: 0.0,
            y_scale: 0.0,
            xz_factor: 0.0,
            y_factor: 0.0,
        };
        if dim == Dimension::End {
            sn.xz_scale = 2.0;
            sn.y_scale = 1.0;
            sn.xz_factor = 80.0;
            sn.y_factor = 160.0;
        } else {
            sn.octsurf = OctaveNoise::new_java(&mut rng, -3, 4);
            // C: skipNextN(&s, 262*10)
            rng.skip(262 * 10);
            sn.octdepth = OctaveNoise::new_java(&mut rng, -15, 16);
            sn.xz_scale = 0.9999999814507745;
            sn.y_scale = 0.9999999814507745;
            sn.xz_factor = 80.0;
            sn.y_factor = 160.0;
        }
        sn
    }

    /// `sampleSurfaceNoise`：三维地形密度采样。
    ///
    /// 返回 `clampedLerp(0.5 + 0.05*main, min/512, max/512)`；正值大致对应
    /// 固体方块。坐标为方块级。
    pub fn sample(&self, x: i32, y: i32, z: i32) -> f64 {
        let xz_scale = 684.412 * self.xz_scale;
        let y_scale = 684.412 * self.y_scale;
        let xz_step = xz_scale / self.xz_factor;
        let y_step = y_scale / self.y_factor;

        let mut min_noise = 0.0;
        let mut max_noise = 0.0;
        let mut main_noise = 0.0;
        let mut persist = 1.0;
        let mut contrib = 1.0;

        for i in 0..16 {
            // maintainPrecision 在 cubiomes 中是恒等函数，略。
            let dx = x as f64 * xz_scale * persist;
            let dy = y as f64 * y_scale * persist;
            let dz = z as f64 * xz_scale * persist;
            let sy = y_scale * persist;
            let ty = y as f64 * sy;

            min_noise += self.octmin.octaves[i].sample(dx, dy, dz, sy, ty) * contrib;
            max_noise += self.octmax.octaves[i].sample(dx, dy, dz, sy, ty) * contrib;

            if i < 8 {
                let dx = x as f64 * xz_step * persist;
                let dy = y as f64 * y_step * persist;
                let dz = z as f64 * xz_step * persist;
                let sy = y_step * persist;
                let ty = y as f64 * sy;
                main_noise += self.octmain.octaves[i].sample(dx, dy, dz, sy, ty) * contrib;
            }
            persist *= 0.5;
            contrib *= 2.0;
        }

        clamped_lerp(
            0.5 + 0.05 * main_noise,
            min_noise / 512.0,
            max_noise / 512.0,
        )
    }

    /// `sampleSurfaceNoiseBetween`：带上下界早退的密度采样。
    ///
    /// 逐 octave（从低频到高频）累加，一旦剩余振幅不可能使结果回到
    /// `[noise_min, noise_max]` 区间就提前返回界值。结果与全量计算的
    /// 钳制值一致（这是 C 的优化，不是近似误差来源）。
    pub fn sample_between(&self, x: i32, y: i32, z: i32, noise_min: f64, noise_max: f64) -> f64 {
        let xz_scale = 684.412 * self.xz_scale;
        let y_scale = 684.412 * self.y_scale;
        let mut vmin = 0.0;
        let mut vmax = 0.0;

        let mut persist = 1.0 / 32768.0;
        let mut amp = 64.0;

        for i in (0..16).rev() {
            let dx = x as f64 * xz_scale * persist;
            let dz = z as f64 * xz_scale * persist;
            let sy = y_scale * persist;
            let dy = y as f64 * sy;

            vmin += self.octmin.octaves[i].sample(dx, dy, dz, sy, dy) * amp;
            vmax += self.octmax.octaves[i].sample(dx, dy, dz, sy, dy) * amp;
            if vmin - amp > noise_max && vmax - amp > noise_max {
                return noise_max;
            }
            if vmin + amp < noise_min && vmax + amp < noise_min {
                return noise_min;
            }

            amp *= 0.5;
            persist *= 2.0;
        }

        let xz_step = xz_scale / self.xz_factor;
        let y_step = y_scale / self.y_factor;
        let mut vmain = 0.5;

        let mut persist = 1.0 / 128.0;
        let mut amp = 0.05 * 128.0;

        for i in (0..8).rev() {
            let dx = x as f64 * xz_step * persist;
            let dz = z as f64 * xz_step * persist;
            let sy = y_step * persist;
            let dy = y as f64 * sy;

            vmain += self.octmain.octaves[i].sample(dx, dy, dz, sy, dy) * amp;
            if vmain - amp > 1.0 {
                return vmax;
            }
            if vmain + amp < 0.0 {
                return vmin;
            }

            amp *= 0.5;
            persist *= 2.0;
        }

        clamped_lerp(vmain, vmin, vmax)
    }
}
