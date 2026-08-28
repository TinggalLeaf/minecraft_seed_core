//! Beta 1.7 及更早版本的群系/地形噪声，移植自 cubiomes `biomenoise.c`
//! 的 `BiomeNoiseBeta` / `SurfaceNoiseBeta` 系列函数。
//!
//! ## 结构
//!
//! - [`BiomeNoiseBeta`]：3 个 [`OctaveNoise`] 气候噪声（温度/湿度/辅助），
//!   采样出口为 64×64 查找表 [`get_old_beta_biome`]。
//! - [`SurfaceNoiseBeta`]：5 组地形倍频（min/max/main/contA/contB），
//!   用于海洋判定与地表高度近似。
//! - [`gen_biome_noise_beta_scaled`]：`genBiomeNoiseBetaScaled`，任意正
//!   scale 的 beta 群系区域生成（scale 1/2 走对角遍历算法）。
//! - [`approx_surface_beta`]：`approxSurfaceBeta`，地表高度近似。
//!
//! ## 与 C 的差异
//!
//! - C 的 `sampleBiomeNoiseBeta` 支持 `nptype`（只算单个气候参数并返回
//!   定点值）以加速群系树查找；本库只需要温度/湿度两个值，未移植该
//!   分支（[`BiomeNoiseBeta::sample_biome_noise_beta`] 恒等价于
//!   `nptype == -1`）。
//! - C 的 `genBiomeNoiseBetaScaled` 在 `out` 缓冲末尾之后复用内存存放
//!   `SeaLevelColumnNoiseBeta` 滚动缓冲；这里用独立的 `Vec`。
//! - C 的 `NO_BETA_OCEAN` 标志（跳过海洋映射）未移植；行为恒等价于
//!   C 的默认（`flags = 0`，启用海洋映射）。

use crate::biome::BiomeId;
use crate::rng::java::JavaRandom;

use super::octave::OctaveNoise;
use crate::generator::Range;

/// `BiomeNoiseBeta`：Beta 1.7 的气候噪声（温度/湿度/辅助三项）。
#[derive(Clone, Debug)]
pub struct BiomeNoiseBeta {
    climate: [OctaveNoise; 3],
}

/// `SeaLevelColumnNoiseBeta`：单列海平面噪声（genColumnNoise 的输出）。
#[derive(Clone, Copy, Debug, Default)]
struct SeaLevelColumnNoiseBeta {
    cont_a: f64,
    cont_b: f64,
    min: [f64; 2],
    max: [f64; 2],
    main: [f64; 2],
}

/// `SurfaceNoiseBeta`：Beta 1.7 的地形倍频噪声组合。
#[derive(Clone, Debug)]
pub struct SurfaceNoiseBeta {
    octmin: OctaveNoise,
    octmax: OctaveNoise,
    octmain: OctaveNoise,
    octcont_a: OctaveNoise,
    octcont_b: OctaveNoise,
}

impl BiomeNoiseBeta {
    /// 占位构造（[`BiomeNoiseBeta::set_beta_biome_seed`] 前不可用）。
    pub fn new_uninit() -> Self {
        let mk = || OctaveNoise::new_beta(&mut JavaRandom::new(0), 1, 1.0, 1.0, 1.0, 1.0);
        BiomeNoiseBeta {
            climate: [mk(), mk(), mk()],
        }
    }

    /// `setBetaBiomeSeed`：用世界种子初始化三项气候噪声。
    pub fn set_beta_biome_seed(&mut self, seed: u64) {
        let mut rng = JavaRandom::new(seed.wrapping_mul(9871) as i64);
        self.climate[0] = OctaveNoise::new_beta(&mut rng, 4, 0.025 / 1.5, 0.25, 0.55, 2.0);
        let mut rng = JavaRandom::new(seed.wrapping_mul(39811) as i64);
        self.climate[1] = OctaveNoise::new_beta(&mut rng, 4, 0.05 / 1.5, 1.0 / 3.0, 0.55, 2.0);
        let mut rng = JavaRandom::new(seed.wrapping_mul(0x84a59) as i64);
        self.climate[2] = OctaveNoise::new_beta(&mut rng, 2, 0.25 / 1.5, 10.0 / 17.0, 0.55, 2.0);
    }

    /// `sampleBiomeNoiseBeta`（`nptype == -1` 分支）：返回
    /// `(群系 ID, [温度, 湿度])`。
    ///
    /// 注意气候噪声按 1:1 比例采样（`x`/`z` 为方块坐标）。
    pub fn sample_biome_noise_beta(&self, x: i32, z: i32) -> (BiomeId, [f64; 2]) {
        let f = self.climate[2].sample_beta17_biome(x as f64, z as f64) * 1.1 + 0.5;

        let mut t = (self.climate[0].sample_beta17_biome(x as f64, z as f64) * 0.15 + 0.7) * 0.99
            + f * 0.01;
        t = 1.0 - (1.0 - t) * (1.0 - t);
        t = t.clamp(0.0, 1.0);

        let mut h = (self.climate[1].sample_beta17_biome(x as f64, z as f64) * 0.15 + 0.5) * 0.998
            + f * 0.002;
        h = h.clamp(0.0, 1.0);

        (get_old_beta_biome(t as f32, h as f32), [t, h])
    }
}

/// `getOldBetaBiome`：温度/湿度经 64×64 表映射到 beta 群系。
///
/// 表中数值 0–9 索引到 `bmap`（plains/desert/forest/taiga/swamp/
/// snowy_tundra/savanna/seasonal_forest/rainforest/shrubland）。
pub fn get_old_beta_biome(t: f32, h: f32) -> BiomeId {
    const BMAP: [BiomeId; 10] = [
        BiomeId::Plains,
        BiomeId::Desert,
        BiomeId::Forest,
        BiomeId::Taiga,
        BiomeId::Swamp,
        BiomeId::SnowyTundra,
        BiomeId::Savanna,
        BiomeId::SeasonalForest,
        BiomeId::Rainforest,
        BiomeId::Shrubland,
    ];
    let idx = (t * 63.0) as i32 + (h * 63.0) as i32 * 64;
    BMAP[BIOME_TABLE_BETA_1_7[idx as usize] as usize]
}

impl SurfaceNoiseBeta {
    /// `initSurfaceNoiseBeta`。
    pub fn new(seed: u64) -> Self {
        let mut rng = JavaRandom::new(seed as i64);
        let octmin = OctaveNoise::new_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let octmax = OctaveNoise::new_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let octmain = OctaveNoise::new_beta(&mut rng, 8, 684.412 / 80.0, 0.5, 1.0, 2.0);
        rng.skip(262 * 8);
        let octcont_a = OctaveNoise::new_beta(&mut rng, 10, 1.121, 0.5, 1.0, 2.0);
        let octcont_b = OctaveNoise::new_beta(&mut rng, 16, 200.0, 0.5, 1.0, 2.0);
        SurfaceNoiseBeta {
            octmin,
            octmax,
            octmain,
            octcont_a,
            octcont_b,
        }
    }
}

/// `genColumnNoise`。
fn gen_column_noise(
    snb: &SurfaceNoiseBeta,
    cx: f64,
    cz: f64,
    lacmin: f64,
) -> SeaLevelColumnNoiseBeta {
    let mut dest = SeaLevelColumnNoiseBeta {
        cont_a: snb.octcont_a.sample_amp(cx, 0.0, cz, 0.0, 0.0, true),
        cont_b: snb.octcont_b.sample_amp(cx, 0.0, cz, 0.0, 0.0, true),
        ..Default::default()
    };
    snb.octmin.sample_beta17_terrain(&mut dest.min, cx, cz, false, lacmin);
    snb.octmax.sample_beta17_terrain(&mut dest.max, cx, cz, false, lacmin);
    snb.octmain.sample_beta17_terrain(&mut dest.main, cx, cz, true, lacmin);
    dest
}

/// `processColumnNoise`。
fn process_column_noise(out: &mut [f64; 2], src: &SeaLevelColumnNoiseBeta, climate: [f64; 2]) {
    let mut humi = 1.0 - climate[0] * climate[1];
    humi *= humi;
    humi *= humi;
    humi = 1.0 - humi;
    let mut cont_a = (src.cont_a + 256.0) / 512.0 * humi;
    cont_a = if cont_a > 1.0 { 1.0 } else { cont_a };
    let mut cont_b = src.cont_b / 8000.0;
    if cont_b < 0.0 {
        cont_b = -cont_b * 0.3;
    }
    cont_b = cont_b * 3.0 - 2.0;
    if cont_b < 0.0 {
        cont_b /= 2.0;
        cont_b = if cont_b < -1.0 { -1.0 / 1.4 / 2.0 } else { cont_b / 1.4 / 2.0 };
        cont_a = 0.0;
    } else {
        cont_b = if cont_b > 1.0 { 1.0 / 8.0 } else { cont_b / 8.0 };
    }
    cont_a = if cont_a < 0.0 { 0.5 } else { cont_a + 0.5 };
    cont_b = (cont_b * 17.0) / 16.0;
    cont_b = 17.0 / 2.0 + cont_b * 4.0;
    let low = src.min;
    let high = src.max;
    let selector = src.main;
    for i in 0..=1usize {
        let mut proc_cont = ((i as f64 + 7.0 - cont_b) * 12.0) / cont_a;
        proc_cont = if proc_cont < 0.0 { proc_cont * 4.0 } else { proc_cont };
        let l_sample = low[i] / 512.0;
        let h_sample = high[i] / 512.0;
        let s_sample = (selector[i] / 10.0 + 1.0) / 2.0;
        let mut choose_lhs = if s_sample < 0.0 {
            l_sample
        } else if s_sample > 1.0 {
            h_sample
        } else {
            l_sample + (h_sample - l_sample) * s_sample
        };
        choose_lhs -= proc_cont;
        out[i] = choose_lhs;
    }
}

/// `lerp4`。
fn lerp4(a: &[f64], b: &[f64], c: &[f64], d: &[f64], dy: f64, dx: f64, dz: f64) -> f64 {
    let b00 = a[0] + (a[1] - a[0]) * dy;
    let b01 = b[0] + (b[1] - b[0]) * dy;
    let b10 = c[0] + (c[1] - c[0]) * dy;
    let b11 = d[0] + (d[1] - d[0]) * dy;
    let b0 = b00 + (b10 - b00) * dz;
    let b1 = b01 + (b11 - b01) * dz;
    b0 + (b1 - b0) * dx
}

/// `approxSurfaceBeta`：近似地表高度（y 坐标）。
pub fn approx_surface_beta(bnb: &BiomeNoiseBeta, snb: &SurfaceNoiseBeta, x: i32, z: i32) -> f64 {
    // C 原注：垂直采样可得更高精度的高度值
    let (_, climate) = bnb.sample_biome_noise_beta(x, z);
    let col_noise = gen_column_noise(snb, x as f64 * 0.25, z as f64 * 0.25, 0.0);
    let mut cols = [0.0; 2];
    process_column_noise(&mut cols, &col_noise, climate);
    63.0 + (cols[0] * 0.125 + cols[1] * 0.875) * 0.5
}

/// `genBiomeNoiseBetaScaled`：生成 beta 群系区域（单个平面，沿 y 复制
/// 由调用方处理——C 的 `genBiomes` 对 beta 同样只生成一个平面）。
///
/// `snb` 为 `None` 或 `scale >= 4` 时走简单逐点路径；scale 1/2 且提供了
/// `snb` 时走对角遍历路径（海洋判定需要四角列噪声插值）。
///
/// # Panics
///
/// `scale` 非正或非 2 的幂，或 `r.sy > 1`（C 对后者直接报错退出）。
pub fn gen_biome_noise_beta_scaled(
    bnb: &BiomeNoiseBeta,
    snb: Option<&SurfaceNoiseBeta>,
    out: &mut [i32],
    r: Range,
) {
    assert!(r.sy <= 1, "gen_biome_noise_beta_scaled: r.sy 必须为 0/1");
    assert!(
        r.scale > 0 && (r.scale & (r.scale - 1)) == 0,
        "gen_biome_noise_beta_scaled: scale 必须是 2 的幂"
    );

    if snb.is_none() || r.scale >= 4 {
        let mid = r.scale >> 1;
        for j in 0..r.sz {
            let z = (r.z + j) * r.scale + mid;
            for i in 0..r.sx {
                let x = (r.x + i) * r.scale + mid;
                let (mut id, climate) = bnb.sample_biome_noise_beta(x, z);
                if let Some(snb) = snb {
                    let col_noise = gen_column_noise(
                        snb,
                        x as f64 * 0.25,
                        z as f64 * 0.25,
                        4.0 / r.scale as f64,
                    );
                    let mut cols = [0.0; 2];
                    process_column_noise(&mut cols, &col_noise, climate);
                    if cols[0] * 0.125 + cols[1] * 0.875 <= 0.0 {
                        id = if climate[0] < 0.5 {
                            BiomeId::FrozenOcean
                        } else {
                            BiomeId::Ocean
                        };
                    }
                }
                out[(j * r.sx + i) as usize] = id as i32;
            }
        }
        return;
    }

    let snb = snb.unwrap();
    let cellwidth = r.scale >> 1;
    let cx1 = r.x >> (2 >> cellwidth);
    let cz1 = r.z >> (2 >> cellwidth);
    let cx2 = cx1 + (r.sx >> (2 >> cellwidth)) + 1;
    let cz2 = cz1 + (r.sz >> (2 >> cellwidth)) + 1;
    let steps = 4 >> cellwidth;
    let (min_dim, max_dim) = if cx2 - cx1 > cz2 - cz1 {
        (cz2 - cz1, cx2 - cx1)
    } else {
        (cx2 - cx1, cz2 - cz1)
    };
    let buf_len = (min_dim * 2 + 1) as usize;

    let mut x_start = cx1;
    let mut z_start = cz1;
    let mut idx = 0usize;
    // C 复用 out 末尾之后的内存做滚动缓冲；这里独立分配
    let mut buf = vec![SeaLevelColumnNoiseBeta::default(); buf_len];
    let mut cols = [0.0f64; 8];
    let off: [i32; 5] = [1, 4, 7, 10, 13];

    // 对角线遍历区域，最小化列噪声滚动缓冲
    for stripe in 0..(max_dim + min_dim - 1) {
        let mut cx = x_start;
        let mut cz = z_start;
        while cx < cx2 && cz >= cz1 {
            let csx = (cx * 4) & !15; // 区块坐标起点
            let csz = (cz * 4) & !15;
            let ci = (cx & 3) as usize;
            let cj = (cz & 3) as usize;

            if stripe == 0 {
                buf[idx] = gen_column_noise(snb, cx as f64, cz as f64, 0.0);
            }
            let (_, climate) = bnb.sample_biome_noise_beta(csx + off[ci], csz + off[cj]);
            let mut tmp = [0.0; 2];
            process_column_noise(&mut tmp, &buf[idx], climate);
            cols[0..2].copy_from_slice(&tmp);

            let idx1 = (idx + min_dim as usize + 1) % buf_len;
            if cz == cz1 {
                buf[idx1] = gen_column_noise(snb, cx as f64 + 1.0, cz as f64, 0.0);
            }
            let (_, climate) = bnb.sample_biome_noise_beta(csx + off[ci + 1], csz + off[cj]);
            process_column_noise(&mut tmp, &buf[idx1], climate);
            cols[2..4].copy_from_slice(&tmp);

            let idx2 = (idx + min_dim as usize) % buf_len;
            if cx == cx1 {
                buf[idx2] = gen_column_noise(snb, cx as f64, cz as f64 + 1.0, 0.0);
            }
            let (_, climate) = bnb.sample_biome_noise_beta(csx + off[ci], csz + off[cj + 1]);
            process_column_noise(&mut tmp, &buf[idx2], climate);
            cols[4..6].copy_from_slice(&tmp);

            buf[idx] = gen_column_noise(snb, cx as f64 + 1.0, cz as f64 + 1.0, 0.0);
            let (_, climate) = bnb.sample_biome_noise_beta(csx + off[ci + 1], csz + off[cj + 1]);
            process_column_noise(&mut tmp, &buf[idx], climate);
            cols[6..8].copy_from_slice(&tmp);

            // scale=1: cellwidth=0, steps=4；scale=2: cellwidth=1, steps=2
            for j in 0..steps {
                let z = cz * steps + j;
                if z < r.z || z >= r.z + r.sz {
                    continue;
                }
                for i in 0..steps {
                    let x = cx * steps + i;
                    if x < r.x || x >= r.x + r.sx {
                        continue;
                    }
                    let mid = r.scale >> 1;
                    let bx = x * r.scale + mid;
                    let bz = z * r.scale + mid;
                    let (mut id, climate) = bnb.sample_biome_noise_beta(bx, bz);
                    let dx = (bx & 3) as f64 * 0.25;
                    let dz = (bz & 3) as f64 * 0.25;
                    if lerp4(&cols[0..2], &cols[2..4], &cols[4..6], &cols[6..8], 7.0 / 8.0, dx, dz)
                        <= 0.0
                    {
                        id = if climate[0] < 0.5 {
                            BiomeId::FrozenOcean
                        } else {
                            BiomeId::Ocean
                        };
                    }
                    out[((z - r.z) as usize * r.sx as usize) + (x - r.x) as usize] = id as i32;
                }
            }

            cx += 1;
            cz -= 1;
            idx = (idx + 1) % buf_len;
        }
        if z_start < cz2 - 1 {
            z_start += 1;
        } else {
            x_start += 1;
        }
        if stripe + 1 < min_dim {
            idx = (idx + (min_dim - stripe - 1) as usize) % buf_len;
        } else if stripe + 1 > max_dim {
            idx = (idx + (stripe - max_dim + 2) as usize) % buf_len;
        } else if x_start > cx1 {
            idx = (idx + 1) % buf_len;
        }
    }
}

/// `biome_table_beta_1_7`：64×64 温度（行内）× 湿度（行）群系表，
/// 数值索引到 [`get_old_beta_biome`] 的 `bmap`。
#[rustfmt::skip]
static BIOME_TABLE_BETA_1_7: [u8; 64 * 64] = [
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,1,1,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,1,1,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,0,0,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,
    9,9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,
    9,9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,
    9,9,9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,
    9,9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,
    9,9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,
    9,9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,
    9,9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,
    9,9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,
    9,9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,
    9,9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,
    9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,
    9,9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,
    9,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,2,2,2,2,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,2,2,2,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,2,2,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,2,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,2,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,2,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,2,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,2,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,2,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,7,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,7,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,2,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,8,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,4,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,8,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,2,4,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,8,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,4,4,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,8,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    2,4,4,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,8,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,
    4,4,4,4,4,4,4,4,4,4,4,4,4,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,8,8,
];
