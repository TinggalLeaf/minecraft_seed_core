//! 地表高度近似：移植 cubiomes `generator.c` 的 `mapApproxHeight`。
//!
//! 按维度/版本分四条路径：
//!
//! - **1.18+ 主世界**：用 1:4 群系噪声点的 depth 参数（`np[NP_DEPTH] /
//!   76.0`）近似地表 y，同时输出群系 ID；
//! - **B1.8–1.17 主世界**：5×5 群系深度/缩放核加权 + `SurfaceNoise` 的
//!   depth 偏移噪声，再以 min/max/main 密度噪声做割线法求零点；
//! - **Beta 1.7- 主世界**：[`approx_surface_beta`]（列噪声插值），
//!   不输出群系 ID（C 的 beta 分支不写 `ids`）；
//! - **末地（1.9+）**：转发 [`EndNoise::map_end_surface_height`]。
//!
//! 下界（`DIM_NETHER`）与 1.8- 末地在 C 中返回错误码（127 / 1），这里
//! 改为 panic（属调用方误用）。

use crate::biome::{biome_depth_and_scale, BiomeId};
use crate::noise::beta::{approx_surface_beta, SurfaceNoiseBeta};
use crate::noise::biome_noise::NP_DEPTH;
use crate::noise::surface::SurfaceNoise;
use crate::version::{Dimension, McVersion};

use super::{v1_18, Generator};

/// `mapApproxHeight` 的输出：地表高度（方块 y，`f32` 与 C 的 `float`
/// 输出逐位一致）与（主世界路径的）1:4 群系 ID。
///
/// 末地路径不生成群系（C 中 `ids` 输出参数在末地分支不被写入），
/// 此时 `ids` 为 `None`。
#[derive(Clone, Debug)]
pub struct ApproxHeight {
    /// 地表高度（方块），索引 `y[j*w+i]`。
    pub y: Vec<f32>,
    /// 1:4 群系 ID（仅主世界路径），索引同 `y`。
    pub ids: Option<Vec<BiomeId>>,
}

impl Generator {
    /// 当前维度/种子的地表噪声（`initSurfaceNoise` 的惰性封装）。
    ///
    /// 首次调用时初始化并缓存，只用群系生成功能的调用方不付出代价。
    /// 更换种子/维度（[`Generator::with_seed`]）后缓存失效。
    ///
    /// # Panics
    ///
    /// 未调用 [`Generator::with_seed`]，或维度为下界（C 的
    /// `mapApproxHeight` 对下界返回错误码 127）。
    pub fn surface_noise(&self) -> &SurfaceNoise {
        self.sn.get_or_init(|| {
            SurfaceNoise::new(
                self.dim.expect("Generator: call with_seed() first"),
                self.seed,
            )
        })
    }

    /// `mapApproxHeight`：近似地表高度图。
    ///
    /// 坐标 `(x, z)` 为 1:4 群系比例（末地为方块级 1:1，见下），输出
    /// `w*h` 格。返回 [`ApproxHeight`]。
    ///
    /// - 主世界：`(x, z)` 是 1:4 比例坐标；
    /// - 末地：C 的 `mapApproxHeight` 以 `scale=4` 调用
    ///   `mapEndSurfaceHeight`，即 `(x, z)` 仍是 1:4 比例坐标，输出高度
    ///   为方块 y；
    /// - 下界 / 1.8- 末地：panic（C 返回错误码）。
    pub fn map_approx_height(&self, x: i32, z: i32, w: i32, h: i32) -> ApproxHeight {
        match self.dim.expect("Generator: call with_seed() first") {
            Dimension::Nether => panic!("mapApproxHeight: 下界无地表高度（C 返回 127）"),
            Dimension::End => {
                assert!(
                    self.mc > McVersion::V1_8,
                    "mapApproxHeight: 1.8 及更早的末地无地表高度（C 返回 1）"
                );
                let en = self.en.as_ref().expect("Generator: call with_seed() first");
                let y = en.map_end_surface_height(self.surface_noise(), x, z, w, h, 4, 0);
                ApproxHeight { y, ids: None }
            }
            Dimension::Overworld => {
                if self.mc >= McVersion::V1_18 {
                    self.approx_height_1_18(x, z, w, h)
                } else if self.mc <= McVersion::B1_7 {
                    self.approx_height_beta(x, z, w, h)
                } else {
                    self.approx_height_legacy(x, z, w, h)
                }
            }
        }
    }

    /// 1.18+ 主世界：`np[NP_DEPTH] / 76.0`。
    fn approx_height_1_18(&self, x: i32, z: i32, w: i32, h: i32) -> ApproxHeight {
        let bn = self.bn.as_ref().expect("Generator: call with_seed() first");
        let mut y = vec![0.0f32; (w * h) as usize];
        let mut ids = vec![BiomeId::None; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let (np, id) = v1_18::sample_biome_noise(bn, x + i, 0, z + j, None, 0);
                ids[(j * w + i) as usize] = id;
                y[(j * w + i) as usize] = (np[NP_DEPTH] as f64 / 76.0) as f32;
            }
        }
        ApproxHeight { y, ids: Some(ids) }
    }

    /// Beta 1.7- 主世界：`approxSurfaceBeta`（C 不写 `ids`，这里 `ids` 为
    /// `None`）。
    fn approx_height_beta(&self, x: i32, z: i32, w: i32, h: i32) -> ApproxHeight {
        let bnb = self.bnb.as_ref().expect("Generator: call with_seed() first");
        let snb = SurfaceNoiseBeta::new(self.seed);
        let mut y = vec![0.0f32; (w * h) as usize];
        for j in 0..h {
            for i in 0..w {
                let sample_x = (x + i) * 4 + 2;
                let sample_z = (z + j) * 4 + 2;
                y[(j * w + i) as usize] = approx_surface_beta(bnb, &snb, sample_x, sample_z) as f32;
            }
        }
        ApproxHeight { y, ids: None }
    }

    /// B1.8–1.17 主世界：深度/缩放核加权 + 密度噪声割线法。
    fn approx_height_legacy(&self, x: i32, z: i32, w: i32, h: i32) -> ApproxHeight {
        // with 10 / (sqrt(i**2 + j**2) + 0.2)；字面量与 C 原文一致
        #[allow(clippy::excessive_precision)]
        const BIOME_KERNEL: [f32; 25] = [
            3.302044127, 4.104975761, 4.545454545, 4.104975761, 3.302044127,
            4.104975761, 6.194967155, 8.333333333, 6.194967155, 4.104975761,
            4.545454545, 8.333333333, 50.00000000, 8.333333333, 4.545454545,
            4.104975761, 6.194967155, 8.333333333, 6.194967155, 4.104975761,
            3.302044127, 4.104975761, 4.545454545, 4.104975761, 3.302044127,
        ];

        let sn = self.surface_noise();
        let n = (w * h) as usize;
        // depth[0..n]、scale[n..2n]（C 用一块 malloc 拆两半）
        let mut depth = vec![0.0f64; 2 * n];
        let (depth, scale) = depth.split_at_mut(n);
        let mut ids = vec![BiomeId::None; n];

        // 注意 C 的区域是 w+5 × h+5（5×5 核只需 +4，多出一行/列是 C 的
        // 宽余，原样保留以保持一致）
        let r = super::Range::new(4, x - 2, z - 2, w + 5, h + 5);
        let cache = self.gen_biomes(r);

        for j in 0..h {
            for i in 0..w {
                let idx = (j * w + i) as usize;
                let id0 = cache[((j + 2) * r.sx + (i + 2)) as usize];
                let (d0, _, _) = biome_depth_and_scale(id0 as i32);

                let mut wt = 0.0f64;
                let mut ws = 0.0f64;
                let mut wd = 0.0f64;
                for jj in 0..5 {
                    for ii in 0..5 {
                        let id = cache[((j + jj) * r.sx + (i + ii)) as usize];
                        let (d, s, _) = biome_depth_and_scale(id as i32);
                        // C: float weight = kernel / (d + 2)（double 计算后
                        // 窄化为 float）
                        let mut weight = (BIOME_KERNEL[(jj * 5 + ii) as usize] as f64
                            / (d + 2.0)) as f32;
                        if d > d0 {
                            weight *= 0.5;
                        }
                        ws += s * weight as f64;
                        wd += d * weight as f64;
                        wt += weight as f64;
                    }
                }
                ws /= wt;
                wd /= wt;
                ws = ws * 0.9 + 0.1;
                wd = (wd * 4.0 - 1.0) / 8.0;
                ws = 96.0 / ws;
                wd *= 17.0 / 64.0;
                depth[idx] = wd;
                scale[idx] = ws;
                ids[idx] = id0;
            }
        }

        let mut y = vec![0.0f32; n];
        for j in 0..h {
            for i in 0..w {
                let idx = (j * w + i) as usize;
                let px = x + i;
                let pz = z + j;
                let mut off = sn.octdepth.sample_amp(
                    (px * 200) as f64,
                    10.0,
                    (pz * 200) as f64,
                    1.0,
                    0.0,
                    true,
                );
                off *= 65535.0 / 8000.0;
                if off < 0.0 {
                    off *= -0.3;
                }
                off = off * 3.0 - 2.0;
                if off > 1.0 {
                    off = 1.0;
                }
                off *= 17.0 / 64.0;
                if off < 0.0 {
                    off *= 1.0 / 28.0;
                } else {
                    off *= 1.0 / 40.0;
                }

                let mut vmin = 0.0;
                let mut vmax = 0.0;
                let mut ytest = 8;
                let mut ymin = 0;
                let mut ymax = 32;
                loop {
                    let mut v = [0.0; 2];
                    for k in 0..2 {
                        let py = ytest + k;
                        let mut n0 = sn.sample(px, py, pz);
                        let fall = 1.0 - (2 * py) as f64 / 32.0 + off - 0.46875;
                        let fall = scale[idx] * (fall + depth[idx]);
                        n0 += if fall > 0.0 { 4.0 * fall } else { fall };
                        v[k as usize] = n0;
                        if n0 >= 0.0 && py > ymin {
                            ymin = py;
                            vmin = n0;
                        }
                        if n0 < 0.0 && py < ymax {
                            ymax = py;
                            vmax = n0;
                        }
                    }
                    let dy = v[0] / (v[0] - v[1]);
                    // 远离零取整
                    let dy = if dy <= 0.0 { dy.floor() } else { dy.ceil() };
                    ytest += dy as i32;
                    if ytest <= ymin {
                        ytest = ymin + 1;
                    }
                    if ytest >= ymax {
                        ytest = ymax - 1;
                    }
                    if ymax - ymin <= 1 {
                        break;
                    }
                }

                y[idx] = (8.0 * (vmin / (vmin - vmax) + ymin as f64)) as f32;
            }
        }
        ApproxHeight { y, ids: Some(ids) }
    }
}
