//! 1.18+ 主世界群系判定与区域生成。
//!
//! 移植 cubiomes `biomenoise.c` 的 `climateToBiome`（含 `get_np_dist` /
//! `get_resulting_node`）与 `genBiomeNoiseScaled` / `genBiomeNoise3D`，
//! 以及 `sampleBiomeNoise` 的群系判定部分（噪声采样见
//! [`crate::noise::biome_noise::BiomeNoise`]）。
//!
//! 群系树数据见 [`super::tables`]（由 cubiomes `tables/btree*.h` 生成）。

use crate::biome::BiomeId;
use crate::noise::biome_noise::{BiomeNoise, SAMPLE_NO_SHIFT};
use crate::version::McVersion;

use super::tables::{biome_tree, BiomeTree};
use super::voronoi::{get_voronoi_src_range, voronoi_access_3d};
use super::Range;

/// `get_np_dist`：噪声点到树节点参数区间的平方距离（u64 环绕语义）。
fn get_np_dist(np: &[u64; 6], bt: &BiomeTree, idx: usize) -> u64 {
    let mut ds = 0u64;
    let node = bt.nodes[idx];
    for (i, &npv) in np.iter().enumerate() {
        let pidx = ((node >> (8 * i)) & 0xFF) as usize;
        let a = npv.wrapping_sub(bt.param[2 * pidx + 1] as i64 as u64);
        let b = (bt.param[2 * pidx] as i64 as u64).wrapping_sub(npv);
        let d = if (a as i64) > 0 {
            a
        } else if (b as i64) > 0 {
            b
        } else {
            0
        };
        ds = ds.wrapping_add(d.wrapping_mul(d));
    }
    ds
}

/// `get_resulting_node`：沿树向下找最近的叶子节点下标。
fn get_resulting_node(
    np: &[u64; 6],
    bt: &BiomeTree,
    idx: usize,
    alt: usize,
    mut ds: u64,
    mut depth: usize,
) -> usize {
    if bt.steps[depth] == 0 {
        return idx;
    }
    let step = loop {
        let step = bt.steps[depth] as usize;
        depth += 1;
        if idx + step < bt.nodes.len() {
            break step;
        }
    };

    let node = bt.nodes[idx];
    let mut inner = (node >> 48) as usize;

    let mut leaf = alt;

    for _ in 0..bt.order {
        let ds_inner = get_np_dist(np, bt, inner);
        if ds_inner < ds {
            let leaf2 = get_resulting_node(np, bt, inner, leaf, ds, depth);
            let ds_leaf2 = if inner == leaf2 {
                ds_inner
            } else {
                get_np_dist(np, bt, leaf2)
            };
            if ds_leaf2 < ds {
                ds = ds_leaf2;
                leaf = leaf2;
            }
        }

        inner += step;
        if inner >= bt.nodes.len() {
            break;
        }
    }
    leaf
}

/// `climateToBiome`：把 6 个气候参数（×10000 定点）映射到主世界群系。
///
/// `np` 顺序为 `[temperature, humidity, continentalness, erosion, depth,
/// weirdness]`。`dat` 是可选的缓存/优化参数（对应 C 的可空 `dat`）：传入时
/// 以缓存的叶子下标为起点加速查找，并回写新下标；`None` 表示完整查找。
pub fn climate_to_biome(mc: McVersion, np: &[i64; 6], dat: Option<&mut u64>) -> BiomeId {
    let bt = biome_tree(mc);
    let np_u: [u64; 6] = [
        np[0] as u64,
        np[1] as u64,
        np[2] as u64,
        np[3] as u64,
        np[4] as u64,
        np[5] as u64,
    ];

    let idx = match dat {
        Some(d) => {
            let alt = *d as usize;
            let ds = get_np_dist(&np_u, &bt, alt);
            let idx = get_resulting_node(&np_u, &bt, 0, alt, ds, 0);
            *d = idx as u64;
            idx
        }
        None => get_resulting_node(&np_u, &bt, 0, 0, u64::MAX, 0),
    };

    BiomeId::from_i32(((bt.nodes[idx] >> 48) & 0xFF) as i32).unwrap_or(BiomeId::None)
}

/// `sampleBiomeNoise`：1.18+ 主世界单点群系采样（1:4 比例坐标）。
///
/// 返回 `(np, biome)`：`np` 为 6 个定点气候值（供测试与调试），`biome`
/// 为最终群系。`dat` / `flags` 语义同 C（flag 常量见
/// [`crate::noise::biome_noise`]）。
pub fn sample_biome_noise(
    bn: &BiomeNoise,
    x: i32,
    y: i32,
    z: i32,
    dat: Option<&mut u64>,
    flags: u32,
) -> ([i64; 6], BiomeId) {
    let np = bn.sample_np(x, y, z, flags);
    let id = climate_to_biome(bn.mc(), &np, dat);
    (np, id)
}

/// `genBiomeNoise3D`：按 cell 逐点采样（`opt` 对应 C 的同名优化开关：
/// 跳过 shift 扰动并启用 `dat` 缓存）。
fn gen_biome_noise_3d(bn: &BiomeNoise, out: &mut [BiomeId], r: Range, opt: bool) {
    let mut dat = 0u64;
    let flags = if opt { SAMPLE_NO_SHIFT } else { 0 };
    let scale = if r.scale > 4 { r.scale / 4 } else { 1 };
    let mid = scale / 2;
    let mut p = 0usize;
    for k in 0..r.sy.max(1) {
        let yk = r.y + k;
        for j in 0..r.sz {
            let zj = (r.z + j) * scale + mid;
            for i in 0..r.sx {
                let xi = (r.x + i) * scale + mid;
                let (_, id) =
                    sample_biome_noise(bn, xi, yk, zj, if opt { Some(&mut dat) } else { None }, flags);
                out[p] = id;
                p += 1;
            }
        }
    }
}

/// `genBiomeNoiseScaled`：1.18+ 主世界区域群系生成。
///
/// `r.scale` 支持 1、4、16、64、256（0 视为 4）。scale 1 使用 voronoi 扰动
/// （需要 `sha`，即 [`super::voronoi::get_voronoi_sha`] 的结果）。输出索引为
/// `out[i_y*sx*sz + i_z*sx + i_x]`。
pub fn gen_biome_noise_scaled(bn: &BiomeNoise, r: Range, sha: u64) -> Vec<BiomeId> {
    let sy = if r.sy == 0 { 1 } else { r.sy };
    let siz = (r.sx * sy * r.sz) as usize;
    let mut out = vec![BiomeId::None; siz];

    if r.scale == 1 {
        let s = get_voronoi_src_range(Range { sy, ..r });
        let src = if siz > 1 {
            // 源区域足够大，一次性生成后查表（对应 C 的 out+siz 缓冲区复用）。
            let mut src = vec![BiomeId::None; (s.sx * s.sy.max(1) * s.sz) as usize];
            gen_biome_noise_3d(bn, &mut src, s, false);
            Some(src)
        } else {
            None
        };

        let mut p = 0usize;
        for k in 0..sy {
            for j in 0..r.sz {
                for i in 0..r.sx {
                    let (x4, y4, z4) = voronoi_access_3d(sha, r.x + i, r.y + k, r.z + j);
                    out[p] = match &src {
                        Some(src) => {
                            let (lx, ly, lz) = (x4 - s.x, y4 - s.y, z4 - s.z);
                            src[(ly * s.sx * s.sz + lz * s.sx + lx) as usize]
                        }
                        None => sample_biome_noise(bn, x4, y4, z4, None, 0).1,
                    };
                    p += 1;
                }
            }
        }
    } else {
        // 高于 1:4 的比例启用有损加速（对应 C 的 MC-241546 注释说明）。
        gen_biome_noise_3d(bn, &mut out, Range { sy, ..r }, r.scale > 4);
    }
    out
}
