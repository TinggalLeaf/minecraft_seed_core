//! 下界群系生成（1.16+ 多噪声）。
//!
//! 移植 cubiomes `biomenoise.c` 的 `setNetherSeed` / `getNetherBiome` /
//! `mapNether3D`（含 `fillRad3D` 填充优化）/ `genNetherScaled`。
//!
//! 下界群系是 3D 的，但实际判定只依赖 temperature/humidity 两个
//! DoublePerlin 噪声（y 被忽略），在 (温度, 湿度) 平面上取最近群系点。
//! 1.15 及更早版本恒为 `nether_wastes`。

use crate::biome::BiomeId;
use crate::noise::DoublePerlinNoise;
use crate::rng::JavaRandom;
use crate::version::McVersion;

use super::voronoi::{get_voronoi_src_range, voronoi_access_3d};
use super::Range;

/// 下界群系生成器（对应 cubiomes `NetherNoise`）。
#[derive(Clone, Debug)]
pub struct NetherNoise {
    pub temperature: DoublePerlinNoise,
    pub humidity: DoublePerlinNoise,
}

impl NetherNoise {
    /// `setNetherSeed`：temperature 用 `seed`，humidity 用 `seed + 1`，
    /// 均为旧版 Java 随机路径的 DoublePerlin（omin=-7, len=2）。
    pub fn new(seed: u64) -> Self {
        let mut rng = JavaRandom::new(seed as i64);
        let temperature = DoublePerlinNoise::new_java(&mut rng, -7, 2);
        let mut rng = JavaRandom::new(seed.wrapping_add(1) as i64);
        let humidity = DoublePerlinNoise::new_java(&mut rng, -7, 2);
        NetherNoise {
            temperature,
            humidity,
        }
    }

    /// `getNetherBiome`：1:4 比例坐标的下界群系（y 不影响结果）。
    ///
    /// 返回 `(biome, ndel)`：`ndel` 为最近与次近群系点的噪声空间距离差
    /// （C 的 `ndel` 输出参数，供 `mapNether3D` 的半径填充优化使用）。
    pub fn get_biome(&self, x: i32, _y: i32, z: i32) -> (BiomeId, f32) {
        // (temp, humidity, 固定权重, biome)
        const NPOINTS: [[f32; 4]; 5] = [
            [0.0, 0.0, 0.0, 8.0],                 // nether_wastes
            [0.0, -0.5, 0.0, 170.0],              // soul_sand_valley
            [0.4, 0.0, 0.0, 171.0],               // crimson_forest
            [0.0, 0.5, 0.375 * 0.375, 172.0],     // warped_forest
            [-0.5, 0.0, 0.175 * 0.175, 173.0],    // basalt_deltas
        ];

        let temp = self.temperature.sample(x as f64, 0.0, z as f64) as f32;
        let humidity = self.humidity.sample(x as f64, 0.0, z as f64) as f32;

        let mut id = 0usize;
        let mut dmin = f32::MAX;
        let mut dmin2 = f32::MAX;
        for (i, np) in NPOINTS.iter().enumerate() {
            let dx = np[0] - temp;
            let dy = np[1] - humidity;
            let dsq = dx * dx + dy * dy + np[2];
            if dsq < dmin {
                dmin2 = dmin;
                dmin = dsq;
                id = i;
            } else if dsq < dmin2 {
                dmin2 = dsq;
            }
        }

        let ndel = dmin2.sqrt() - dmin.sqrt();
        let biome = BiomeId::from_i32(NPOINTS[id][3] as i32).unwrap_or(BiomeId::None);
        (biome, ndel)
    }

    /// `genNetherScaled`：下界区域群系生成。
    ///
    /// `r.scale` 支持 1、4、16、64、256（<=0 视为 4，1 以外按 1:4 的整数倍）。
    /// `mc <= 1.15` 时全部填充 `nether_wastes`（C 的 genNetherScaled 行为）。
    /// scale 1 使用 voronoi 扰动（`sha` 为世界种子的 voronoi 散列）。
    pub fn gen_scaled(&self, r: Range, mc: McVersion, sha: u64) -> Vec<BiomeId> {
        let mut r = r;
        if r.scale <= 0 {
            r.scale = 4;
        }
        if r.sy == 0 {
            r.sy = 1;
        }
        let siz = (r.sx * r.sy * r.sz) as usize;

        if mc <= McVersion::V1_15 {
            return vec![BiomeId::NetherWastes; siz];
        }

        if r.scale == 1 {
            let s = get_voronoi_src_range(r);
            let src = if siz > 1 {
                Some(map_nether_3d(self, s, 1.0))
            } else {
                None
            };

            let mut out = vec![BiomeId::None; siz];
            let mut p = 0usize;
            for k in 0..r.sy {
                for j in 0..r.sz {
                    for i in 0..r.sx {
                        let (x4, y4, z4) =
                            voronoi_access_3d(sha, r.x + i, r.y + k, r.z + j);
                        out[p] = match &src {
                            Some(src) => {
                                let (lx, ly, lz) = (x4 - s.x, y4 - s.y, z4 - s.z);
                                src[(ly * s.sx * s.sz + lz * s.sx + lx) as usize]
                            }
                            None => self.get_biome(x4, y4, z4).0,
                        };
                        p += 1;
                    }
                }
            }
            out
        } else {
            map_nether_3d(self, r, 1.0)
        }
    }
}

/// `fillRad3D`：以 (x,y,z) 为中心、半径 `rad` 的球内填入 `id`。
#[allow(clippy::too_many_arguments)]
fn fill_rad_3d(
    out: &mut [BiomeId],
    x: i32,
    y: i32,
    z: i32,
    sx: i32,
    sy: i32,
    sz: i32,
    id: BiomeId,
    rad: f32,
) {
    let r = rad as i32;
    if r <= 0 {
        return;
    }
    let rsq = (rad * rad).floor() as i32;

    for k in -r..=r {
        let ak = y + k;
        if !(0..sy).contains(&ak) {
            continue;
        }
        let ksq = k * k;
        let base = (ak * sx * sz) as usize;

        for j in -r..=r {
            let aj = z + j;
            if !(0..sz).contains(&aj) {
                continue;
            }
            let jksq = j * j + ksq;
            for i in -r..=r {
                let ai = x + i;
                if !(0..sx).contains(&ai) {
                    continue;
                }
                let ijksq = i * i + jksq;
                if ijksq > rsq {
                    continue;
                }
                out[base + (aj * sx + ai) as usize] = id;
            }
        }
    }
}

/// `mapNether3D`：带半径填充优化的 3D 区域生成（`confidence` 恒为 1.0，
/// 即精确模式；C 中 <1.0 的近似模式未移植）。
fn map_nether_3d(nn: &NetherNoise, r: Range, confidence: f32) -> Vec<BiomeId> {
    assert!(r.scale > 3, "mapNether3D() invalid scale for this function");
    let scale = r.scale / 4;

    // None 即 C 中的 0（未填充标记）；BiomeId::None 判别值为 -1，
    // 不会与真实群系混淆。
    let mut out = vec![BiomeId::None; (r.sx * r.sy * r.sz) as usize];

    // 噪声增量除以最大梯度（~0.05）得到同群系体素的最小直径。
    let invgrad = 1.0 / (confidence * 0.05 * 2.0) / scale as f32;

    for k in 0..r.sy {
        for j in 0..r.sz {
            for i in 0..r.sx {
                let idx = (k * r.sx * r.sz + j * r.sx + i) as usize;
                if out[idx] != BiomeId::None {
                    continue;
                }
                let xi = (r.x + i) * scale;
                let yk = r.y + k;
                let zj = (r.z + j) * scale;
                let (v, noisedelta) = nn.get_biome(xi, yk, zj);
                out[idx] = v;
                let cellrad = noisedelta * invgrad;
                fill_rad_3d(&mut out, i, j, k, r.sx, r.sy, r.sz, v, cellrad);
            }
        }
    }
    out
}
