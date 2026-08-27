//! 末地群系生成（1.9+ simplex 高地噪声）。
//!
//! 移植 cubiomes `biomenoise.c` 的 `setEndSeed` / `mapEndBiome` /
//! `getEndBiome` / `mapEnd` / `getEndHeightNoise` 与 `genEndScaled`。
//!
//! 末地群系是 2D 的：以 simplex 噪声在 1:16 比例上生成"高地"高度图，
//! 再以 25×25 细胞窗口内的加权距离判定 highlands/midlands/barrens/
//! small_end_islands；中心 64×64 细胞（1:16 比例）恒为 `the_end`。
//!
//! ## 未覆盖
//!
//! `genEndScaled` 的 scale 1 路径（1:1 voronoi 缩放，依赖 layers.c 的
//! `mapVoronoi114` / `mapVoronoiPlane`）未移植，调用会 panic。scale 4/16/64
//! 均可用。`mapEndSurfaceHeight` / `getEndSurfaceHeight`（地表高度查询，
//! 依赖 `SurfaceNoise`）不在群系生成范围内，未移植。

use crate::biome::BiomeId;
use crate::noise::PerlinNoise;
use crate::rng::JavaRandom;
use crate::version::McVersion;

use super::Range;

/// 末地群系生成器（对应 cubiomes `EndNoise`）。
#[derive(Clone, Debug)]
pub struct EndNoise {
    pub perlin: PerlinNoise,
    mc: McVersion,
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
        }
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
    /// `r.scale` 支持 4、16、64、256（以及其它 >16 的比例，走高度噪声路径）。
    /// `sy > 1` 时把 2D 结果沿 y 复制（末地群系是 2D 的）。
    /// `mc <= 1.8` 时全部填充 `the_end`。
    ///
    /// # Panics
    ///
    /// `r.scale == 1`（1:1 voronoi 缩放未移植，见模块文档）。
    pub fn gen_scaled(&self, r: Range, mc: McVersion) -> Vec<BiomeId> {
        let mut r = r;
        if r.sy == 0 {
            r.sy = 1;
        }

        if mc <= McVersion::V1_8 {
            return vec![BiomeId::TheEnd; (r.sx * r.sy * r.sz) as usize];
        }

        let mut out = if r.scale == 1 {
            panic!("genEndScaled: scale 1:1 voronoi for the End is not ported yet");
        } else if r.scale == 4 {
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
