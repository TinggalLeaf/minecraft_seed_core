//! 结构查找核心：结构类型配置、候选区块位置、变体、群系可行性、
//! 要塞迭代器与出生点估计。
//!
//! 逐函数移植自 cubiomes 的 `finders.c` / `finders.h`，保证与 Web 端
//! （mcseedmap.com 的 WASM 后端）结果一致。
//!
//! - [`config`]：结构类型枚举与按版本的 salt / spacing 配置表。
//! - [`region`]：region 种子派生、候选区块坐标计算、史莱姆区块等。
//! - [`variant`]：结构变体（村庄样式、堡垒类型等）。
//! - [`viability`]：群系层面的可行性判定（不含地形高度检查）。
//! - [`stronghold`]：要塞迭代器（1.8 及以前与 1.9+ 两代环带算法）。
//! - [`spawn`]：出生点估计（世界群系搜索 + 适应度评估）。
//!
//! 未移植项（依赖噪声管线或属于其他模块）：`getSpawn` 的精确地形采样、
//! `isViableStructureTerrain` / `isViableEndCityTerrain`、结构部件生成
//! （`getEndCityPieces` / `getFortressPieces` / 村庄 `getHouseList`）、
//! `quadbase.c` 与 `biomfilter.c`。

pub mod config;
pub mod region;
pub mod spawn;
pub mod stronghold;
pub mod variant;
pub mod viability;

pub use config::{get_config, StructureConfig, StructureType, FEATURE_NUM};
pub use region::{
    chunk_generate_rnd, get_end_islands, get_feature_chunk_in_region, get_feature_pos,
    get_large_structure_chunk_in_region, get_large_structure_pos, get_mineshafts,
    get_population_seed, get_structure_pos, is_slime_chunk, move_structure, EndIsland, Pos,
};
pub use spawn::{estimate_spawn, locate_biome};
pub use stronghold::{init_first_stronghold, is_stronghold_biome, StrongholdIter};
pub use variant::{get_variant, StructureVariant};
pub use viability::{
    biome_exists, is_overworld, is_viable_feature_biome, is_viable_structure_pos,
};

#[cfg(test)]
mod tests;
