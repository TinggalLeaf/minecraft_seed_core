//! 噪声模块（Perlin / Octave / DoublePerlin）。
//!
//! 逐函数移植 cubiomes `noise.c` / `noise.h`：
//!
//! - [`perlin`]：单频 Perlin 与 2D Simplex（Beta 1.7 群系用）。
//! - [`octave`]：倍频叠加（旧版 `octaveInit`、Beta `octaveInitBeta`、
//!   1.18+ `xOctaveInit`）。
//! - [`double_perlin`]：DoublePerlin（1.18+ 气候噪声的基本构件）。
//! - [`biome_noise`]：1.18+ 主世界群系气候噪声（6 参数 DoublePerlin 组合
//!   + depth spline 表）。
//! - [`beta`]：Beta 1.7 及更早的气候噪声（[`BiomeNoiseBeta`]）与地形
//!   倍频（[`SurfaceNoiseBeta`]）。
//! - [`surface`]：三维地形密度噪声（`SurfaceNoise`，1.18- 主世界与末地
//!   的地表高度近似 / 末地城地形可行性用）。
//!
//! 所有采样函数与 C 参考实现逐位一致（`f64` 位级相等），唯一刻意的偏差是
//! `sample_beta17_terrain` 修正了 cubiomes 原版的越界读，详见
//! [`perlin`] 模块文档。

pub mod double_perlin;
pub mod beta;
pub mod biome_noise;
pub mod octave;
pub mod perlin;
pub mod surface;

pub use beta::{approx_surface_beta, get_old_beta_biome, BiomeNoiseBeta, SurfaceNoiseBeta};
pub use biome_noise::BiomeNoise;
pub use double_perlin::DoublePerlinNoise;
pub use octave::OctaveNoise;
pub use perlin::PerlinNoise;
pub use surface::SurfaceNoise;
