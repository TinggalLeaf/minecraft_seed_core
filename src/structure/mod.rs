//! 结构查找核心：结构类型配置、候选区块位置、变体、群系可行性、
//! 要塞迭代器与出生点估计。
//!
//! 逐函数移植自 cubiomes 的 `finders.c` / `finders.h`，保证与 Web 端
//! （mcseedmap.com 的 WASM 后端）结果一致。
//!
//! - [`config`]：结构类型枚举与按版本的 salt / spacing 配置表。
//! - [`region`]：region 种子派生、候选区块坐标计算、史莱姆区块等。
//! - [`variant`]：结构变体（村庄样式、堡垒类型等）。
//! - [`viability`]：可行性判定（群系层面 + 地形级
//!   `isViableStructureTerrain` / `isViableEndCityTerrain` /
//!   `isEndChunkEmpty`）。
//! - [`stronghold`]：要塞迭代器（1.8 及以前与 1.9+ 两代环带算法）。
//! - [`spawn`]：出生点（估计 `estimateSpawn` + 精确 `getSpawn`）。
//! - [`gateway`]：末地折跃门落点（`getLinkedGatewayChunk/Pos`）。
//! - [`pieces`]：结构部件生成（`getEndCityPieces` / `getFortressPieces` /
//!   村庄 `getHouseList`）。
//! - [`quadbase`]：四连底座高速搜索（`isQuadBase*` 系列、`scanForQuads`、
//!   `searchAll48`、`getOptimalAfk`）。
//!
//! 未移植项：`biomfilter.c`。

pub mod config;
pub mod fossil;
pub mod gateway;
pub mod pieces;
pub mod quadbase;
pub mod region;
pub mod spawn;
pub mod stronghold;
pub mod variant;
pub mod viability;

pub use config::{get_config, StructureConfig, StructureType, FEATURE_NUM};
pub use fossil::{get_fossil_positions, scan_fossils, FOSSIL_RARITY, FOSSIL_SALTS};
pub use gateway::{
    get_linked_gateway_chunk, get_linked_gateway_pos, map_end_island_height,
};
pub use pieces::{
    end_city, fortress, get_end_city_pieces, get_fortress_pieces, get_house_list, house,
    HouseList, Piece, Pos3,
};
pub use quadbase::{
    LOW20_QUAD_CLASSIC, LOW20_QUAD_HUT_BARELY, LOW20_QUAD_HUT_NORMAL, LOW20_QUAD_IDEAL,
    QuadHutCst, get_optimal_afk, get_quad_hut_cst, is_quad_base, is_quad_base_feature,
    is_quad_base_feature24, is_quad_base_feature24_classic, is_quad_base_large, scan_for_quads,
    search_all48,
};
pub use region::{
    chunk_generate_rnd, get_end_islands, get_feature_chunk_in_region, get_feature_pos,
    get_large_structure_chunk_in_region, get_large_structure_pos, get_mineshafts,
    get_population_seed, get_structure_pos, is_slime_chunk, move_structure, EndIsland, Pos,
};
pub use spawn::{estimate_spawn, get_spawn, locate_biome};
pub use stronghold::{init_first_stronghold, is_stronghold_biome, StrongholdIter};
pub use variant::{get_variant, StructureVariant};
pub use viability::{
    biome_exists, is_end_chunk_empty, is_overworld, is_viable_end_city_terrain,
    is_viable_feature_biome, is_viable_structure_pos, is_viable_structure_terrain,
};

// ============================================================================
// 并行结构扫描
// ============================================================================

use crate::generator::Generator;
use crate::version::{Dimension, McVersion};

/// 并行扫描 region 范围内的**可行**结构位置（候选 + 群系可行性过滤）。
///
/// 等价于对每个 region 调用 [`get_structure_pos`] + [`is_viable_structure_pos`]，
/// 但按 `reg_z` 条纹切分到 `threads` 个线程，每线程持有独立的
/// [`Generator`]（Generator 非 `Sync`，不能跨线程共享）。返回顺序与
/// 单线程 `(rz, rx)` 行优先遍历一致。
///
/// 零依赖（仅 std）。`threads <= 1` 时退化为单线程。
///
/// ```
/// use minecraft_seed_core::{Dimension, McVersion, StructureType};
/// use minecraft_seed_core::structure::find_structures_par;
///
/// // 1.21.4 主世界 ±4 region 内的村庄
/// let villages = find_structures_par(
///     StructureType::Village, McVersion::V1_21, Dimension::Overworld,
///     12345, -4..=4, -4..=4, 4,
/// );
/// assert!(!villages.is_empty());
/// ```
pub fn find_structures_par(
    stype: config::StructureType,
    mc: McVersion,
    dim: Dimension,
    seed: u64,
    reg_x: std::ops::RangeInclusive<i32>,
    reg_z: std::ops::RangeInclusive<i32>,
    threads: usize,
) -> Vec<region::Pos> {
    let (rx0, rx1) = (*reg_x.start(), *reg_x.end());
    let (rz0, rz1) = (*reg_z.start(), *reg_z.end());
    let n_rz = (rz1 - rz0 + 1).max(0) as usize;
    if threads <= 1 || n_rz < 2 {
        let g = Generator::new(mc).with_seed(dim, seed);
        return scan_stripe(stype, &g, seed, rx0, rx1, rz0, rz1);
    }
    let threads = threads.min(n_rz);
    let chunk = n_rz.div_ceil(threads);
    let mut parts = Vec::with_capacity(threads);
    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(threads);
        for t in 0..threads {
            let z0 = rz0 + (t * chunk) as i32;
            let z1 = (z0 + chunk as i32 - 1).min(rz1);
            if z0 > z1 {
                break;
            }
            handles.push(s.spawn(move || {
                let g = Generator::new(mc).with_seed(dim, seed);
                scan_stripe(stype, &g, seed, rx0, rx1, z0, z1)
            }));
        }
        for h in handles {
            parts.push(h.join().unwrap());
        }
    });
    let mut out = Vec::new();
    for part in parts {
        out.extend(part);
    }
    out
}

/// 单条纹扫描（[`find_structures_par`] 的工作单元）。
fn scan_stripe(
    stype: config::StructureType,
    g: &Generator,
    seed: u64,
    rx0: i32,
    rx1: i32,
    rz0: i32,
    rz1: i32,
) -> Vec<region::Pos> {
    let mut out = Vec::new();
    for rz in rz0..=rz1 {
        for rx in rx0..=rx1 {
            if let Some(pos) = region::get_structure_pos(stype, g.version(), seed, rx, rz) {
                if viability::is_viable_structure_pos(stype, g, pos.x, pos.z, 0) != 0 {
                    out.push(pos);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
