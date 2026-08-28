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
pub mod gateway;
pub mod pieces;
pub mod quadbase;
pub mod region;
pub mod spawn;
pub mod stronghold;
pub mod variant;
pub mod viability;

pub use config::{get_config, StructureConfig, StructureType, FEATURE_NUM};
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

#[cfg(test)]
mod tests;
