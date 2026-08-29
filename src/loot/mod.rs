//! Minecraft 战利品表（loot table）模块，当前数据版本 1.20.1。
//!
//! ## 模块结构
//!
//! - [`json`]：零依赖迷你 JSON 解析器，仅支持 loot table 实际用到的语法子集。
//! - [`rng`]：在 crate 根 [`crate::rng`] 上的薄封装，提供 [`LootRng`] trait
//!   与 1.18+ / 1.17- 两种实现。
//! - [`table`]：1:1 翻译 Python `src/loot_table.py` 的生成算法；暴露
//!   [`LootTable`] / [`LootItem`]。支持 `item` / `alternatives` /
//!   `sequence` / `group` entry 与 `set_count` / `enchant_randomly` /
//!   `apply_bonus`（时运）函数。
//! - [`seeds`]：1:1 翻译 Python `src/structure_seeds.py`，覆盖 36 个
//!   chest id 的 5 种推导模式。
//! - [`stats`]：1:1 翻译 Python `src/loot_stats.py`，提供解析概率
//!   （PMF / 期望值）与蒙特卡洛采样统计。
//! - [`registry`]：按 [`LootVersion`] 路由到对应版本的静态注册表
//!   （`registry/v<version>.rs` 一版本一文件；数据一致的版本只做
//!   re-export 复用，不复制 JSON——如 [`LootVersion::V1_20_4`] 复用
//!   1.20.1 的全部 853 张表：43 chests、88 entities、20 gameplay、
//!   6 archaeology、695 blocks、1 empty），数据来自 `data/loot/1.20.1/`。
//!
//! ## 数据布局
//!
//! 每个版本独占一个目录：
//!
//! ```text
//! data/loot/<version>/{chests,entities,gameplay,archaeology,blocks}/*.json
//! ```
//!
//! JSON 在编译期用 `include_str!` 嵌入二进制，对应源数据来自上游 Python
//! 项目 `E:\Projects\Minecraft\宝箱内容生成\data\`，请勿手工编辑；
//! 注册表由 `scripts/gen_registry.py` 生成。
//!
//! ## 端到端示例
//!
//! ```
//! use minecraft_seed_core::loot::{LootVersion, chest_rng};
//!
//! let v = LootVersion::V1_20_1;
//! let table = v.get("minecraft:chests/ruined_portal").unwrap();
//! let mut rng = chest_rng(12345, "minecraft:chests/ruined_portal", 100, 64, 200);
//! let items = table.generate(&mut rng, 0.0);
//! assert!(!items.is_empty());
//! ```

pub mod json;
pub mod registry;
pub mod rng;
pub mod seeds;
pub mod stats;
pub mod table;

pub use registry::LootVersion;
pub use rng::{LegacyLootRng, LootRng, XoroshiroLootRng};
pub use seeds::{chest_rng, derive_seed};
pub use stats::{
    aggregate_counts, analyse_table, contains_query, contains_query_par, simulate, simulate_par,
    ContainsResult, ItemAgg, TableStats,
};
pub use table::{LootItem, LootTable};

/// 预测一个箱子（chest 类表）的内容。
///
/// 等价于 Python `loot_predictor.py` 的 `predict`：`chest_seed_for` 推导
/// 种子 → xoroshiro → 预消费一次 `next_long` → `table.generate`。
pub fn predict_chest(
    version: LootVersion,
    chest: &str,
    world_seed: i64,
    x: i32,
    y: i32,
    z: i32,
) -> Result<Vec<LootItem>, String> {
    let id = version
        .lookup(chest)
        .ok_or_else(|| format!("unknown loot table: {chest}"))?;
    let table = version.get(id)?;
    let mut rng = chest_rng(world_seed, id, x, y, z);
    Ok(table.generate(&mut rng, 0.0))
}

/// 预测非 chest 类表（entities / blocks / gameplay / archaeology）的一次产出。
///
/// 等价于 Python `predict_loot_table`：RNG 种子为 `world_seed + sample_index`。
pub fn predict_table(
    version: LootVersion,
    table_id: &str,
    world_seed: i64,
    sample_index: i64,
) -> Result<Vec<LootItem>, String> {
    let id = version
        .lookup(table_id)
        .ok_or_else(|| format!("unknown loot table: {table_id}"))?;
    let table = version.get(id)?;
    let mut rng = XoroshiroLootRng::new(world_seed.wrapping_add(sample_index) as u64);
    Ok(table.generate(&mut rng, 0.0))
}

/// 解析分析（期望数量 / 单次命中概率），等价 Python `--analyze`。
pub fn analyze(version: LootVersion, table_id: &str) -> Result<TableStats, String> {
    let id = version
        .lookup(table_id)
        .ok_or_else(|| format!("unknown loot table: {table_id}"))?;
    let table = version.get(id)?;
    Ok(analyse_table(id, &table))
}

/// 某物品在表中出现的概率（蒙特卡洛），等价 Python `--contains`。
pub fn contains_probability(
    version: LootVersion,
    table_id: &str,
    item: &str,
    world_seed: i64,
    samples: usize,
) -> Result<ContainsResult, String> {
    let id = version
        .lookup(table_id)
        .ok_or_else(|| format!("unknown loot table: {table_id}"))?;
    let table = version.get(id)?;
    Ok(contains_query(&table, item, world_seed, samples, 0, 64, 0))
}
