//! 末地群系生成（1.9+ simplex 高地噪声）。
//!
//! 移植 cubiomes `biomenoise.c` 的 `setEndSeed` / `mapEndBiome` /
//! `getEndBiome` / `mapEnd` / `getEndHeightNoise` 与 `genEndScaled`
//!（含 scale 1 的 voronoi 缩放：1.9–1.14 走 layers.c 的 `mapVoronoi114`
//! 旧版平面算法，1.15+ 走 `mapVoronoiPlane` 的 SHA 三维变体），以及地表
//! 高度部分：`sampleNoiseColumnEnd` / `getSurfaceHeight` /
//! `mapEndSurfaceHeight` / `getEndSurfaceHeight`（依赖
//! [`crate::noise::SurfaceNoise`]）。
//!
//! 末地群系是 2D 的：以 simplex 噪声在 1:16 比例上生成"高地"高度图，
//! 再以 25×25 细胞窗口内的加权距离判定 highlands/midlands/barrens/
//! small_end_islands；中心 64×64 细胞（1:16 比例）恒为 `the_end`。

use crate::biome::BiomeId;
use crate::noise::PerlinNoise;
use crate::rng::seed::layer_salt;
use crate::rng::JavaRandom;
use crate::version::McVersion;

use super::voronoi::{
    get_voronoi_sha, get_voronoi_src_range, map_voronoi_114_plane, map_voronoi_plane,
};
use super::Range;

/// 末地群系生成器（对应 cubiomes `EndNoise`）。
#[derive(Clone, Debug)]
pub struct EndNoise {
    pub perlin: PerlinNoise,
    mc: McVersion,
    /// 世界种子（C 中 `genEndScaled` 的 `sha` 参数来自
    /// `applySeed` 的 `getVoronoiSHA(seed)`，这里存种子按需计算）。
    seed: u64,
}

impl EndNoise {
    /// `setEndSeed`：先跳过 17292 次随机数再初始化 Perlin
    /// （对齐 MC 中末地噪声在随机流中的位置）。
    pub fn new(mc: McVersion, seed: u64) -> Self {
        let mut rng = JavaRandom::new(seed as i64);
        rng.skip(17292);
        EndNoise {
            perlin: PerlinNoise::new_java(&mut rng),
            mc,
            seed,
        }
    }

    /// 版本（`setEndSeed` 记录的 `en->mc`）。
    pub fn mc(&self) -> McVersion {
        self.mc
    }

    /// `mapEndBiome`：1:16 比例（chunk 级）的末地群系区域。
    ///
    /// 输出 `w*h` 个群系，索引为 `out[j*w+i]`。
    pub fn map_end_biome(&self, x: i32, z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let hw = (w + 26) as i64;
        let hh = (h + 26) as i64;
        let mut hmap = vec![0u16; (hw * hh) as usize];

        for j in 0..hh {
            for i in 0..hw {
                let rx = x as i64 + i - 12;
                let rz = z as i64 + j - 12;
                let rsq = (rx * rx + rz * rz) as u64;
                let mut v = 0u16;
                if rsq > 4096 && self.perlin.sample_simplex_2d(rx as f64, rz as f64) < -0.9 {
                    let b = (((rx as f32).abs() * 3439.0 + (rz as f32).abs() * 147.0) as u32) % 13 + 9;
                    v = (b * b) as u16;
                }
                hmap[(j * hw + i) as usize] = v;
            }
        }

        let mut out = vec![BiomeId::None; (w as i64 * h as i64) as usize];
        for j in 0..h as i64 {
            for i in 0..w as i64 {
                let mut hx = i + x as i64;
                let mut hz = j + z as i64;
                let rsq = (hx * hx + hz * hz) as u64;

                if rsq <= 4096 {
                    out[(j * w as i64 + i) as usize] = BiomeId::TheEnd;
                } else {
                    hx = 2 * hx + 1;
                    hz = 2 * hz + 1;
                    if self.mc > McVersion::V1_13 {
                        // 外岛环带（1.14+）：rsq 低 32 位符号位置位时视为 barrens
                        let rsq = (hx * hx + hz * hz) as u64;
                        if (rsq as i32) < 0 {
                            out[(j * w as i64 + i) as usize] = BiomeId::EndBarrens;
                            continue;
                        }
                    }
                    let off = ((hz / 2 - z as i64) * hw + (hx / 2 - x as i64)) as usize;
                    out[(j * w as i64 + i) as usize] =
                        get_end_biome(hx as i32, hz as i32, &hmap[off..], hw as i32);
                }
            }
        }
        out
    }

    /// `mapEnd`：1:4 比例的末地群系区域（内部按 1:16 生成后放大）。
    pub fn map_end(&self, x: i32, z: i32, w: i32, h: i32) -> Vec<BiomeId> {
        let cx = x >> 2;
        let cz = z >> 2;
        let cw = ((x + w) >> 2) + 1 - cx;
        let ch = ((z + h) >> 2) + 1 - cz;

        let buf = self.map_end_biome(cx, cz, cw, ch);

        let mut out = vec![BiomeId::None; (w * h) as usize];
        for j in 0..h {
            let cj = ((z + j) >> 2) - cz;
            for i in 0..w {
                let ci = ((x + i) >> 2) - cx;
                out[(j * w + i) as usize] = buf[(cj * cw + ci) as usize];
            }
        }
        out
    }

    /// `getEndHeightNoise`：末地高度噪声（8 格/细胞）。
    /// `range == 0` 时默认采样 12 细胞半径。
    pub fn get_end_height_noise(&self, x: i32, z: i32, range: i32) -> f32 {
        let hx = x / 2;
        let hz = z / 2;
        let oddx = x % 2;
        let oddz = z % 2;

        let mut h = 64 * (x as i64 * x as i64 + z as i64 * z as i64);
        let range = if range == 0 { 12 } else { range };

        for j in -range..=range {
            for i in -range..=range {
                let mut rx = (hx + i) as i64;
                let mut rz = (hz + j) as i64;
                let mut rsq = (rx * rx + rz * rz) as u64;
                if rsq > 4096 && self.perlin.sample_simplex_2d(rx as f64, rz as f64) < -0.9 {
                    let v = (((rx as f32).abs() * 3439.0 + (rz as f32).abs() * 147.0) as u32) % 13 + 9;
                    rx = (oddx - i * 2) as i64;
                    rz = (oddz - j * 2) as i64;
                    rsq = (rx * rx + rz * rz) as u64;
                    let noise = rsq.wrapping_mul((v as i64 * v as i64) as u64) as i64;
                    if noise < h {
                        h = noise;
                    }
                }
            }
        }

        let ret = 100.0 - (h as f32).sqrt();
        ret.clamp(-100.0, 80.0)
    }

    /// `genEndScaled`：末地区域群系生成。
    ///
    /// `r.scale` 支持 1、4、16、64、256（以及其它 >16 的比例，走高度噪声
    /// 路径）。scale 1 为 1:1 voronoi 缩放：1.9–1.14 用旧版平面算法
    /// （`mapVoronoi114`，零初始化层 + `startSalt = getLayerSalt(10)`），
    /// 1.15+ 用 SHA 播种的三维变体（`mapVoronoiPlane`，逐 y 层计算）。
    /// `sy > 1` 时，scale != 1 或 mc <= 1.14 把 2D 结果沿 y 复制（末地
    /// 群系在旧版是 2D 的）；1.15+ 的 scale 1 每层独立计算。
    /// `mc <= 1.8` 时全部填充 `the_end`。
    pub fn gen_scaled(&self, r: Range, mc: McVersion) -> Vec<BiomeId> {
        let mut r = r;
        if r.sy == 0 {
            r.sy = 1;
        }

        if mc <= McVersion::V1_8 {
            return vec![BiomeId::TheEnd; (r.sx * r.sy * r.sz) as usize];
        }

        if r.scale == 1 {
            let s = get_voronoi_src_range(r);
            let src: Vec<i32> = self
                .map_end(s.x, s.z, s.sx, s.sz)
                .into_iter()
                .map(|b| b as i32)
                .collect();
            let plane = (r.sx * r.sz) as usize;

            if mc <= McVersion::V1_14 {
                // 1.9–1.14：平面 voronoi（C 中零初始化层，startSalt =
                // getLayerSalt(10)，startSeed = 0），随后沿 y 复制
                let mut out = vec![0i32; plane];
                map_voronoi_114_plane(
                    layer_salt(10),
                    0,
                    &src,
                    &mut out,
                    r.x,
                    r.z,
                    r.sx,
                    r.sz,
                );
                let mut out: Vec<BiomeId> = out
                    .into_iter()
                    .map(|v| {
                        BiomeId::from_i32(v)
                            .unwrap_or_else(|| panic!("EndNoise: 未知末地群系 ID {v}"))
                    })
                    .collect();
                out.reserve(plane * (r.sy - 1) as usize);
                for _ in 1..r.sy {
                    let layer = out[..plane].to_vec();
                    out.extend_from_slice(&layer);
                }
                return out;
            }

            // 1.15+：voronoi 在末地沿 y 变化（C 此处直接返回，不做 2D 复制）
            let sha = get_voronoi_sha(self.seed);
            let mut out = vec![0i32; plane * r.sy as usize];
            for iy in 0..r.sy {
                map_voronoi_plane(
                    sha,
                    &mut out[iy as usize * plane..(iy as usize + 1) * plane],
                    &src,
                    r.x,
                    r.z,
                    r.sx,
                    r.sz,
                    r.y + iy,
                    s.x,
                    s.z,
                    s.sx,
                    s.sz,
                );
            }
            return out
                .into_iter()
                .map(|v| {
                    BiomeId::from_i32(v)
                        .unwrap_or_else(|| panic!("EndNoise: 未知末地群系 ID {v}"))
                })
                .collect();
        }

        let mut out = if r.scale == 4 {
            self.map_end(r.x, r.z, r.sx, r.sz)
        } else if r.scale == 16 {
            self.map_end_biome(r.x, r.z, r.sx, r.sz)
        } else {
            let d = r.scale as f32 / 8.0;
            let mut out = vec![BiomeId::None; (r.sx * r.sz) as usize];
            for j in 0..r.sz {
                for i in 0..r.sx {
                    let hx = ((i + r.x) as f32 * d) as i64;
                    let hz = ((j + r.z) as f32 * d) as i64;
                    let rsq = (hx * hx + hz * hz) as u64;
                    if rsq <= 16384 {
                        out[(j * r.sx + i) as usize] = BiomeId::TheEnd;
                        continue;
                    } else if mc > McVersion::V1_13 && (rsq as i32) < 0 {
                        out[(j * r.sx + i) as usize] = BiomeId::EndBarrens;
                        continue;
                    }
                    let h = self.get_end_height_noise(hx as i32, hz as i32, 4);
                    out[(j * r.sx + i) as usize] = if h > 40.0 {
                        BiomeId::EndHighlands
                    } else if h >= 0.0 {
                        BiomeId::EndMidlands
                    } else if h >= -20.0 {
                        BiomeId::EndBarrens
                    } else {
                        BiomeId::SmallEndIslands
                    };
                }
            }
            out
        };

        // 2D 沿 y 复制为 3D
        let siz = (r.sx * r.sz) as usize;
        out.reserve(siz * (r.sy - 1) as usize);
        for _ in 1..r.sy {
            let layer = out[..siz].to_vec();
            out.extend_from_slice(&layer);
        }
        out
    }
}

/// `getEndBiome`：以 25×25 高度图窗口的加权最小距离判定末地群系。
/// `hx`/`hz` 为奇数化的双倍坐标，`hmap` 从对应细胞起始，`hw` 为行宽。
fn get_end_biome(hx: i32, hz: i32, hmap: &[u16], hw: i32) -> BiomeId {
    // (25-2*i)*(25-2*i)
    const DS: [u16; 26] = [
        625, 529, 441, 361, 289, 225, 169, 121, 81, 49, 25, 9, 1,
        1, 9, 25, 49, 81, 121, 169, 225, 289, 361, 441, 529, 625,
    ];

    let p_dsi = if hx < 0 { 1usize } else { 0 };
    let p_dsj = if hz < 0 { 1usize } else { 0 };

    let mut h: u32 = if hx.abs() <= 15 && hz.abs() <= 15 {
        64 * (hx * hx + hz * hz) as u32
    } else {
        14401
    };

    for j in 0..25usize {
        let dsj = DS[p_dsj + j] as u32;
        let row = j * hw as usize;
        for i in 0..25usize {
            let e = hmap[row + i] as u32;
            if e != 0 {
                let u = (DS[p_dsi + i] as u32 + dsj) * e;
                if u < h {
                    h = u;
                }
            }
        }
    }

    if h < 3600 {
        BiomeId::EndHighlands
    } else if h <= 10000 {
        BiomeId::EndMidlands
    } else if h <= 14400 {
        BiomeId::EndBarrens
    } else {
        BiomeId::SmallEndIslands
    }
}

// ============================================================================
// 末地地表高度（依赖 SurfaceNoise）
// ============================================================================

use crate::noise::surface::SurfaceNoise;

/// `sampleNoiseColumnEnd` 与 `isEndChunkEmpty` 共用的上界衰减表：
/// `clamp((32 + 46 - y) / 64.0, 0, 1)`（y = 0..=32）。
pub(crate) const END_UPPER_DROP: [f64; 33] = [
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, // 0-7
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 63.0 / 64.0, // 8-15
    62.0 / 64.0, 61.0 / 64.0, 60.0 / 64.0, 59.0 / 64.0, 58.0 / 64.0, 57.0 / 64.0, 56.0 / 64.0,
    55.0 / 64.0, // 16-23
    54.0 / 64.0, 53.0 / 64.0, 52.0 / 64.0, 51.0 / 64.0, 50.0 / 64.0, 49.0 / 64.0, 48.0 / 64.0,
    47.0 / 64.0, // 24-31
    46.0 / 64.0, // 32
];

/// 下界衰减表：`clamp((y - 1) / 7.0, 0, 1)`（y = 0..=32）。
pub(crate) const END_LOWER_DROP: [f64; 33] = [
    0.0, 0.0, 1.0 / 7.0, 2.0 / 7.0, 3.0 / 7.0, 4.0 / 7.0, 5.0 / 7.0, 6.0 / 7.0, // 0-7
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, // 8-15
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, // 16-23
    1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, // 24-31
    1.0, // 32
];

/// C `util.h` 的 `floordiv`：向下取整除法（`b > 0`）。
#[inline]
pub(crate) fn floordiv(a: i32, b: i32) -> i32 {
    let q = a / b;
    if a % b != 0 && ((a ^ b) < 0) { q - 1 } else { q }
}

/// `rng.h` 的 `lerp` / `lerp2` / `lerp3`（保持 C 的参数序与运算序）。
#[inline(always)]
fn lerp(part: f64, from: f64, to: f64) -> f64 {
    from + part * (to - from)
}

#[inline(always)]
fn lerp2(dx: f64, dy: f64, v00: f64, v10: f64, v01: f64, v11: f64) -> f64 {
    lerp(dy, lerp(dx, v00, v10), lerp(dx, v01, v11))
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn lerp3(
    dx: f64,
    dy: f64,
    dz: f64,
    v000: f64,
    v100: f64,
    v010: f64,
    v110: f64,
    v001: f64,
    v101: f64,
    v011: f64,
    v111: f64,
) -> f64 {
    let v000 = lerp2(dx, dy, v000, v100, v010, v110);
    let v001 = lerp2(dx, dy, v001, v101, v011, v111);
    lerp(dz, v000, v001)
}

impl EndNoise {
    /// `sampleNoiseColumnEnd`：末地 8 格细胞 `(x, z)` 处的噪声列
    /// `y = colymin..=colymax`（含地表衰减钳制）。
    ///
    /// `out` 长度须为 `colymax - colymin + 1`。1.14+ 的外岛环带之外
    /// （`rsq` 低 32 位符号位置位）填充 NaN——下游 `get_surface_height`
    /// 的比较对 NaN 恒假，效果与 C 一致（视为无地表）。
    pub(crate) fn sample_noise_column_end(
        &self,
        sn: &SurfaceNoise,
        out: &mut [f64],
        x: i32,
        z: i32,
        colymin: i32,
        colymax: i32,
    ) {
        if self.mc > McVersion::V1_13 {
            // 外岛环带
            let rsq = (x as u64)
                .wrapping_mul(x as u64)
                .wrapping_add((z as u64).wrapping_mul(z as u64));
            if ((rsq as u32) as i32) < 0 {
                out[..=(colymax - colymin) as usize].fill(f64::NAN);
                return;
            }
        }

        // depth ∈ [-108, +72]，noise ∈ [-128, +128]（C 注释的推导略）
        let depth = (self.get_end_height_noise(x, z, 0) - 8.0) as f64;
        for y in colymin..=colymax {
            if END_LOWER_DROP[y as usize] == 0.0 {
                out[(y - colymin) as usize] = -30.0;
                continue;
            }
            let noise = sn.sample_between(x, y, z, -128.0, 128.0);
            let clamped = noise + depth;
            let clamped = lerp(END_UPPER_DROP[y as usize], -3000.0, clamped);
            let clamped = lerp(END_LOWER_DROP[y as usize], -30.0, clamped);
            out[(y - colymin) as usize] = clamped;
        }
    }

    /// `mapEndSurfaceHeight`：区域末地地表高度（方块 y）。
    ///
    /// 坐标 `(x, z)` 与输出索引 `out[j*w+i]` 按 `scale`（支持 1/2/4/8）
    /// 缩放；`ymin` 是调用方已知的下界剪枝（`>> 2` 后钳到 [2, 17]）。
    ///
    /// # Panics
    ///
    /// `scale` 不在 {1, 2, 4, 8} 中（C 返回 1 错误码）。
    #[allow(clippy::too_many_arguments)] // 与 C 参数列表一一对应
    pub fn map_end_surface_height(
        &self,
        sn: &SurfaceNoise,
        x: i32,
        z: i32,
        w: i32,
        h: i32,
        scale: i32,
        ymin: i32,
    ) -> Vec<f32> {
        assert!(
            matches!(scale, 1 | 2 | 4 | 8),
            "mapEndSurfaceHeight: 不支持的 scale {scale}"
        );

        let y0 = (ymin >> 2).clamp(2, 17);
        let y1 = 18;
        let yn = (y1 - y0 + 1) as usize;
        let cellmid = if scale > 1 { scale as f64 / 16.0 } else { 0.0 };
        let cellsiz = 8 / scale;
        let cx = floordiv(x, cellsiz);
        let cz = floordiv(z, cellsiz);
        let cw = floordiv(x + w - 1, cellsiz) - cx + 2;

        let mut buf = vec![0.0f64; yn * cw as usize * 2];
        let (mut ncol0, mut ncol1) = buf.split_at_mut(yn * cw as usize);
        for i in 0..cw as usize {
            self.sample_noise_column_end(sn, &mut ncol1[i * yn..(i + 1) * yn], cx + i as i32, cz, y0, y1);
        }

        let mut y = vec![0.0f32; (w * h) as usize];
        for j in 0..h {
            let cj = floordiv(z + j, cellsiz);
            let dj = z + j - cj * cellsiz;
            if j == 0 || dj == 0 {
                std::mem::swap(&mut ncol0, &mut ncol1);
                for i in 0..cw as usize {
                    self.sample_noise_column_end(
                        sn,
                        &mut ncol1[i * yn..(i + 1) * yn],
                        cx + i as i32,
                        cj + 1,
                        y0,
                        y1,
                    );
                }
            }

            for i in 0..w {
                let ci = floordiv(x + i, cellsiz);
                let di = x + i - ci * cellsiz;
                let dx = di as f64 / cellsiz as f64 + cellmid;
                let dz = dj as f64 / cellsiz as f64 + cellmid;
                let off = ((ci - cx) as usize) * yn;
                let ncol00 = &ncol0[off..off + yn];
                let ncol01 = &ncol1[off..off + yn];
                let ncol10 = &ncol0[off + yn..off + 2 * yn];
                let ncol11 = &ncol1[off + yn..off + 2 * yn];
                y[(j * w + i) as usize] = get_surface_height(
                    ncol00, ncol01, ncol10, ncol11, y0, y1, 4, dx, dz,
                ) as f32;
            }
        }
        y
    }
}

/// `getSurfaceHeight`：由四角噪声列与分数偏移 `(dx, dz)` 插值求地表高度
/// （自上往下第一个插值噪声 > 0 的方块）。
///
/// 噪声列切片长度须为 `colymax - colymin + 1`；全 NaN 列（外岛环带外）
/// 返回 0，与 C 行为一致。
#[allow(clippy::too_many_arguments)]
pub(crate) fn get_surface_height(
    ncol00: &[f64],
    ncol01: &[f64],
    ncol10: &[f64],
    ncol11: &[f64],
    colymin: i32,
    colymax: i32,
    blockspercell: i32,
    dx: f64,
    dz: f64,
) -> i32 {
    for celly in (colymin..colymax).rev() {
        let idx = (celly - colymin) as usize;
        let v000 = ncol00[idx];
        let v001 = ncol01[idx];
        let v100 = ncol10[idx];
        let v101 = ncol11[idx];
        let v010 = ncol00[idx + 1];
        let v011 = ncol01[idx + 1];
        let v110 = ncol10[idx + 1];
        let v111 = ncol11[idx + 1];

        for y in (0..blockspercell).rev() {
            let dy = y as f64 / blockspercell as f64;
            // 注意：C 注释强调这里不是 x, y, z 的顺序
            let noise = lerp3(
                dy, dx, dz, v000, v010, v100, v110, v001, v011, v101, v111,
            );
            if noise > 0.0 {
                return celly * blockspercell + y;
            }
        }
    }
    0
}

/// `getEndSurfaceHeight`：单点末地地表高度（方块坐标）。
pub fn get_end_surface_height(mc: McVersion, seed: u64, x: i32, z: i32) -> i32 {
    let en = EndNoise::new(mc, seed);
    let sn = SurfaceNoise::new(crate::version::Dimension::End, seed);

    // 末地噪声列在 8 格细胞网格上变化
    let cellx = x >> 3;
    let cellz = z >> 3;
    let dx = (x & 7) as f64 / 8.0;
    let dz = (z & 7) as f64 / 8.0;

    const Y0: i32 = 0;
    const Y1: i32 = 32;
    const YN: usize = (Y1 - Y0 + 1) as usize;
    let mut ncol00 = [0.0; YN];
    let mut ncol01 = [0.0; YN];
    let mut ncol10 = [0.0; YN];
    let mut ncol11 = [0.0; YN];
    en.sample_noise_column_end(&sn, &mut ncol00, cellx, cellz, Y0, Y1);
    en.sample_noise_column_end(&sn, &mut ncol01, cellx, cellz + 1, Y0, Y1);
    en.sample_noise_column_end(&sn, &mut ncol10, cellx + 1, cellz, Y0, Y1);
    en.sample_noise_column_end(&sn, &mut ncol11, cellx + 1, cellz + 1, Y0, Y1);

    get_surface_height(&ncol00, &ncol01, &ncol10, &ncol11, Y0, Y1, 4, dx, dz)
}
