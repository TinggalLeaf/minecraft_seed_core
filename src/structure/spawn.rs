//! 出生点估计：移植 cubiomes `finders.c` 的 `locateBiome` 与
//! `estimateSpawn`（含 1.18+ 的适应度搜索 `findFittestPos`）。
//!
//! **未覆盖**：`getSpawn`（精确出生点）依赖 `SurfaceNoise` 与
//! `mapApproxHeight`（generator.c 的地表高度近似），该部分噪声管线尚未
//! 移植，见模块汇报。mcseedmap 使用的近似出生点即 [`estimate_spawn`]。

use crate::biome::BiomeId;
use crate::generator::v1_18::sample_biome_noise;
use crate::generator::{Generator, Range};
use crate::noise::biome_noise::SAMPLE_NO_DEPTH;
use crate::rng::JavaRandom;
use crate::version::McVersion;

use super::region::Pos;
use super::viability::id_set_test;

const PI: f64 = std::f64::consts::PI;

/// `locateBiome`：在 `(x, z)` 周围边长 `2*radius+1` 的方形区域内找一个
/// 可行群系的伪随机位置。
///
/// - 坐标为方块级；1.18+ 内部按 1:4 采样（半径同样除 4）。
/// - `valid_b`/`valid_m`：群系位集（id 0–63 与 128–191）。
/// - `rng`：用于多点等概率抽取的随机源（调用方以 `JavaRandom::new(seed)`
///   初始化；要塞迭代器等会持续推进同一状态）。
/// - `passes`：输出可行群系数（C 的可空输出参数）。
///
/// 与 C 一致：找不到可行群系时返回原点 `(x, z)`。
#[allow(clippy::too_many_arguments)]
pub fn locate_biome(
    g: &Generator,
    x: i32,
    y: i32,
    z: i32,
    radius: i32,
    valid_b: u64,
    valid_m: u64,
    rng: &mut JavaRandom,
    passes: Option<&mut i32>,
) -> Pos {
    let mut out = Pos { x, z };
    let mut found = 0i32;

    if g.version() >= McVersion::V1_18 {
        let x = x >> 2;
        let z = z >> 2;
        let radius = radius >> 2;
        let bn = g.biome_noise().expect("1.18+ 主世界应有 BiomeNoise");
        // dat 缓存贯穿整个扫描（与 C 一致；仅为加速，结果与全查找相同）
        let mut dat = 0u64;
        for j in -radius..=radius {
            for i in -radius..=radius {
                // 模拟顺序相关的群系生成 MC-241546
                let (_, id) = sample_biome_noise(bn, x + i, y, z + j, Some(&mut dat), 0);
                let id = id as i32;
                if !id_set_test(valid_b, valid_m, id) {
                    continue;
                }
                if found == 0 || rng.next_int_bound(found + 1) == 0 {
                    out.x = (x + i) * 4;
                    out.z = (z + j) * 4;
                }
                found += 1;
            }
        }
    } else {
        let x1 = (x - radius) >> 2;
        let z1 = (z - radius) >> 2;
        let x2 = (x + radius) >> 2;
        let z2 = (z + radius) >> 2;
        let width = x2 - x1 + 1;
        let height = z2 - z1 + 1;

        let ids = g.gen_biomes(Range::new(4, x1, z1, width, height).with_y(y, 1));

        if g.version() >= McVersion::V1_13 {
            let mut j = 2;
            for (i, &id) in ids.iter().enumerate() {
                let id = id as i32;
                if !id_set_test(valid_b, valid_m, id) {
                    continue;
                }
                let i = i as i32;
                if found == 0 || rng.next_int_bound(j) == 0 {
                    if found != 0 {
                        j += 1;
                    }
                    out.x = (x1 + i % width) * 4;
                    out.z = (z1 + i / width) * 4;
                    found = 1;
                } else if found != 0 {
                    j += 1;
                }
            }
            found = j - 2;
        } else {
            for (i, &id) in ids.iter().enumerate() {
                let id = id as i32;
                if !id_set_test(valid_b, valid_m, id) {
                    continue;
                }
                let i = i as i32;
                if found == 0 || rng.next_int_bound(found + 1) == 0 {
                    out.x = (x1 + i % width) * 4;
                    out.z = (z1 + i / width) * 4;
                    found += 1;
                }
            }
        }
    }

    if let Some(p) = passes {
        *p = found;
    }
    out
}

// ============================================================================
// 1.18+ 出生点：适应度搜索
// ============================================================================

/// `calcFitness`：出生点适应度（越小越好）。气候参数越出界惩罚越大，
/// 并随距原点距离增大。
fn calc_fitness(g: &Generator, x: i32, z: i32) -> u64 {
    let bn = g.biome_noise().expect("1.18+ 主世界应有 BiomeNoise");
    // C: SAMPLE_NO_DEPTH | SAMPLE_NO_BIOME；后者只跳过群系映射，
    // 本实现的 sample_np 本就不做映射，二者等价
    let np = bn.sample_np(x >> 2, 0, z >> 2, SAMPLE_NO_DEPTH);
    // [6] 是第二个噪声点的 weirdness 区间（译自 C 注释）
    const SPAWN_NP: [[i64; 2]; 7] = [
        [-10000, 10000],
        [-10000, 10000],
        [-1100, 10000],
        [-10000, 10000],
        [0, 0],
        [-10000, -1600],
        [1600, 10000],
    ];

    // C 在 uint64 上回绕计算后与 0 比较有符号性
    let excess = |v: i64, lo: i64, hi: i64| -> u64 {
        let a = v.wrapping_sub(hi);
        let b = (-v).wrapping_add(lo);
        if a > 0 {
            a as u64
        } else if b > 0 {
            b as u64
        } else {
            0
        }
    };

    let mut ds: u64 = 0;
    for i in 0..5 {
        let q = excess(np[i], SPAWN_NP[i][0], SPAWN_NP[i][1]);
        ds = ds.wrapping_add(q.wrapping_mul(q));
    }
    let q = excess(np[5], SPAWN_NP[5][0], SPAWN_NP[5][1]);
    let ds1 = ds.wrapping_add(q.wrapping_mul(q));
    let q = excess(np[5], SPAWN_NP[6][0], SPAWN_NP[6][1]);
    let ds2 = ds.wrapping_add(q.wrapping_mul(q));
    let ds = ds1.min(ds2);

    // 与原点距离的惩罚项
    let a = (x as i64).wrapping_mul(x as i64) as u64;
    let b = (z as i64).wrapping_mul(z as i64) as u64;
    if g.version() <= McVersion::V1_21_1 {
        let s = (a.wrapping_add(b)) as f64 / (2500.0 * 2500.0);
        (s * s * 1e8) as u64 + ds
    } else {
        ds.wrapping_mul(2048 * 2048).wrapping_add(a).wrapping_add(b)
    }
}

/// `findFittest`：绕当前位置做同心圆扫描，把适应度最低点写入 `pos`。
fn find_fittest(g: &Generator, pos: &mut Pos, fitness: &mut u64, maxrad: f64, step: f64) {
    let p = *pos;
    let mut rad = step;
    while rad <= maxrad {
        let mut ang = 0.0;
        while ang <= PI * 2.0 {
            let x = p.x + (ang.sin() * rad) as i32;
            let z = p.z + (ang.cos() * rad) as i32;
            let fit = calc_fitness(g, x, z);
            // 更低（更好）时更新
            if fit < *fitness {
                pos.x = x;
                pos.z = z;
                *fitness = fit;
            }
            ang += step / rad;
        }
        rad += step;
    }
}

/// `findFittestPos`：1.18+ 的近似出生点。
fn find_fittest_pos(g: &Generator) -> Pos {
    let mut spawn = Pos { x: 0, z: 0 };
    let mut fitness = calc_fitness(g, 0, 0);
    find_fittest(g, &mut spawn, &mut fitness, 2048.0, 512.0);
    find_fittest(g, &mut spawn, &mut fitness, 512.0, 32.0);
    // 区块中心
    spawn.x = (spawn.x & !15) + 8;
    spawn.z = (spawn.z & !15) + 8;
    spawn
}

// 1.17- 的可出生群系（C 的 g_spawn_biomes_17）
const SPAWN_BIOMES_17: u64 = (1u64 << BiomeId::Forest as i32)
    | (1u64 << BiomeId::Plains as i32)
    | (1u64 << BiomeId::Taiga as i32)
    | (1u64 << BiomeId::TaigaHills as i32)
    | (1u64 << BiomeId::WoodedHills as i32)
    | (1u64 << BiomeId::Jungle as i32)
    | (1u64 << BiomeId::JungleHills as i32);

/// `estimateSpawn`：世界的近似出生点。
///
/// - 1.7–1.17：在 ±256 方块内随机选取可行群系位置（`locateBiome`），
///   找不到时退回 `(8, 8)`；
/// - 1.18+：气候参数适应度搜索。
///
/// 生成器须已按主世界与目标种子初始化。
pub fn estimate_spawn(g: &Generator) -> Pos {
    estimate_spawn_rng(g).0
}

/// 同 [`estimate_spawn`]，附带推进后的随机状态（C 的 `rng` 输出参数）。
pub(crate) fn estimate_spawn_rng(g: &Generator) -> (Pos, JavaRandom) {
    if g.version() <= McVersion::V1_17 {
        // C 中 `mc <= MC_1_0` 的可行集更窄，本库版本下界 1.7，恒用 1.7+ 集合
        let mut s = JavaRandom::new(g.seed() as i64);
        let mut found = 0;
        let mut spawn = locate_biome(g, 0, 63, 0, 256, SPAWN_BIOMES_17, 0, &mut s, Some(&mut found));
        if found == 0 {
            spawn.x = 8;
            spawn.z = 8;
        }
        (spawn, s)
    } else {
        (find_fittest_pos(g), JavaRandom::new(g.seed() as i64))
    }
}
