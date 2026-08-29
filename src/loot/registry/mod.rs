//! 跨版本 loot table 注册表：按 [`LootVersion`] 把表 id 派发到对应版本
//! 的静态 JSON 源。
//!
//! ## 多版本组织
//!
//! 每个版本一个文件：`v1_20_1.rs` 这类由 `scripts/gen_registry.py`
//! 生成（包含 `TABLES` / `SHORT_NAMES` / `get_raw` / `lookup_short`，
//! JSON 用 `include_str!` 编译期嵌入）。
//!
//! 若若干版本的战利品表**完全一致**，不复制数据：新版本的文件只做
//! re-export，例如 `v1_20_4.rs` 整体转发自 `v1_20_1`——`include_str!`
//! 只出现一次，二进制中只存一份 JSON。
//!
//! 新增版本时：
//! 1. 数据有变化：在 `data/loot/<version>/` 放置该版本 JSON，用
//!    `scripts/gen_registry.py` 生成 `registry/v<version>.rs`；
//! 2. 数据无变化：新建 `registry/v<version>.rs`，内容为一行
//!    `pub use super::v<x>;` 式 re-export（参照 `v1_20_4.rs`）；
//! 3. 在 [`LootVersion`] 加一项并在下面的 match 分派中注册。

use crate::loot::table::LootTable;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub(crate) mod v1_20_1;
pub(crate) mod v1_20_4;

/// 已解析 [`LootTable`] 的全局缓存（解析 JSON 比生成物品慢一个数量级，
/// 重复使用时务必走 [`LootVersion::get_cached`]）。
static TABLE_CACHE: OnceLock<Mutex<HashMap<(LootVersion, &'static str), Arc<LootTable>>>> =
    OnceLock::new();

/// 支持的 Minecraft 版本（与 [`crate::version::McVersion`] 平行，仅覆盖
/// loot table 真正可用的版本范围）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum LootVersion {
    /// Minecraft 1.20.1（截至当前数据快照，853 张原版表：43 chests、
    /// 88 entities、20 gameplay、6 archaeology、695 blocks、1 empty）。
    V1_20_1,
    /// Minecraft 1.20.2–1.20.4：战利品表与 1.20.1 完全一致
    /// （这些补丁版本未动 loot table），数据直接复用 [`V1_20_1`](Self::V1_20_1)，
    /// 不复制。
    V1_20_4,
}

impl LootVersion {
    /// 该版本下全部表 id 的元数据表。
    pub fn tables(self) -> &'static [(&'static str, &'static str)] {
        match self {
            LootVersion::V1_20_1 => v1_20_1::TABLES,
            LootVersion::V1_20_4 => v1_20_4::TABLES,
        }
    }

    /// 按表 id 返回原始 JSON 字符串。
    pub fn get_raw(self, loot_table_id: &str) -> Option<&'static str> {
        match self {
            LootVersion::V1_20_1 => v1_20_1::get_raw(loot_table_id),
            LootVersion::V1_20_4 => v1_20_4::get_raw(loot_table_id),
        }
    }

    /// 按表 id 返回已解析的 [`LootTable`]。
    ///
    /// 每次调用都重新解析 JSON；频繁生成同一张表时用 [`Self::get_cached`]。
    pub fn get(self, loot_table_id: &str) -> Result<LootTable, String> {
        let raw = self
            .get_raw(loot_table_id)
            .ok_or_else(|| format!("unknown loot table id: {loot_table_id}"))?;
        LootTable::from_json_str(raw)
    }

    /// 按表 id 返回已解析并**缓存**的 [`LootTable`]（共享指针，克隆廉价）。
    ///
    /// 首次调用解析并缓存，之后同 `(版本, id)` 直接命中。多线程安全。
    pub fn get_cached(self, loot_table_id: &str) -> Result<Arc<LootTable>, String> {
        let id = self
            .lookup(loot_table_id)
            .ok_or_else(|| format!("unknown loot table id: {loot_table_id}"))?;
        let cache = TABLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = cache.lock().unwrap();
        if let Some(t) = guard.get(&(self, id)) {
            return Ok(Arc::clone(t));
        }
        let table = Arc::new(self.get(id)?);
        guard.insert((self, id), Arc::clone(&table));
        Ok(table)
    }

    /// 短名 → 全 id（对齐 Python `loot_registry.lookup`）。
    ///
    /// 接受：`minecraft:chests/simple_dungeon`、`chests/simple_dungeon`、
    /// 短名（`dungeon`、`ruined_portal`、`sheep_blue` 等）。
    pub fn lookup(self, name: &str) -> Option<&'static str> {
        let s = name.trim();
        if s.is_empty() {
            return None;
        }
        let find = |want: &str| {
            self.tables()
                .iter()
                .find(|(id, _)| *id == want)
                .map(|(id, _)| *id)
        };
        if let Some(id) = find(s) {
            return Some(id);
        }
        if let Some(bare) = s.strip_prefix("minecraft:") {
            if let Some(id) = find(bare) {
                return Some(id);
            }
        } else {
            let cand = format!("minecraft:{s}");
            if let Some(id) = find(&cand) {
                return Some(id);
            }
        }
        match self {
            LootVersion::V1_20_1 => v1_20_1::lookup_short(s),
            LootVersion::V1_20_4 => v1_20_4::lookup_short(s),
        }
    }

    /// 该版本的短名别名表 `(short_name, loot_table_id)`。
    pub fn short_names(self) -> &'static [(&'static str, &'static str)] {
        match self {
            LootVersion::V1_20_1 => v1_20_1::SHORT_NAMES,
            LootVersion::V1_20_4 => v1_20_4::SHORT_NAMES,
        }
    }

    /// 按类别列出表 id。类别与 Python `CATEGORIES` 一致：
    /// `chests` / `entities` / `gameplay` / `archaeology` / `blocks`。
    pub fn category(self, category: &str) -> Vec<&'static str> {
        let prefix = format!("minecraft:{category}/");
        self.tables()
            .iter()
            .map(|(id, _)| *id)
            .filter(move |id| id.starts_with(&prefix))
            .collect()
    }

    /// 所有已注册版本，按枚举顺序升序。
    pub const ALL: &'static [LootVersion] = &[LootVersion::V1_20_1, LootVersion::V1_20_4];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_853_tables() {
        // 1 empty + 43 chests（含 15 village）+ 88 entities（72 + 16 sheep）
        // + 20 gameplay + 6 archaeology + 695 blocks
        assert_eq!(LootVersion::V1_20_1.tables().len(), 853);
    }

    #[test]
    fn lookup_resolves_short_and_full() {
        let v = LootVersion::V1_20_1;
        assert_eq!(v.lookup("dungeon"), Some("minecraft:chests/simple_dungeon"));
        assert_eq!(v.lookup("minecraft:chests/simple_dungeon"),
                   Some("minecraft:chests/simple_dungeon"));
        assert_eq!(v.lookup("zombie"), Some("minecraft:entities/zombie"));
        assert_eq!(v.lookup("sheep_blue"), Some("minecraft:entities/sheep/blue"));
        assert_eq!(v.lookup("oak_log"), Some("minecraft:blocks/oak_log"));
        assert_eq!(v.lookup("does_not_exist"), None);
    }

    #[test]
    fn category_counts_match_python() {
        // 与 Python `CATEGORIES` 一致：chests 含 chests/village/ 子目录。
        let v = LootVersion::V1_20_1;
        assert_eq!(v.category("chests").len(), 43);
        assert_eq!(v.category("entities").len(), 88);
        assert_eq!(v.category("gameplay").len(), 20);
        assert_eq!(v.category("archaeology").len(), 6);
        assert_eq!(v.category("blocks").len(), 695);
    }

    #[test]
    fn every_table_parses() {
        // 对齐 Python `--validate`：每张表都能解析并成功 generate 一次。
        let v = LootVersion::V1_20_1;
        for (id, _) in v.tables() {
            let table = v.get(id).unwrap_or_else(|e| panic!("{id}: {e}"));
            let mut rng = crate::loot::rng::XoroshiroLootRng::new(42);
            let _ = table.generate(&mut rng, 0.0);
        }
    }

    #[test]
    fn empty_table_resolves() {
        let t = LootVersion::V1_20_1.get("minecraft:empty").unwrap();
        let mut rng = crate::loot::rng::XoroshiroLootRng::new(0);
        let out = t.generate(&mut rng, 0.0);
        assert!(out.is_empty());
    }

    #[test]
    fn unknown_id_returns_err() {
        let err = LootVersion::V1_20_1.get("minecraft:does_not_exist").unwrap_err();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn v1_20_4_reuses_v1_20_1_without_copying() {        // 1.20.2–1.20.4 与 1.20.1 战利品表一致：静态表必须是同一份
        // （指针相等证明没有复制数据）。
        assert!(std::ptr::eq(
            LootVersion::V1_20_1.tables(),
            LootVersion::V1_20_4.tables()
        ));
        assert!(std::ptr::eq(
            LootVersion::V1_20_1.short_names(),
            LootVersion::V1_20_4.short_names()
        ));
        assert!(std::ptr::eq(
            LootVersion::V1_20_1.get_raw("minecraft:chests/ruined_portal").unwrap(),
            LootVersion::V1_20_4.get_raw("minecraft:chests/ruined_portal").unwrap()
        ));
        assert_eq!(LootVersion::ALL.len(), 2);
    }

    #[test]
    fn get_cached_shares_parsed_table() {
        let v = LootVersion::V1_20_1;
        let a = v.get_cached("ruined_portal").unwrap();
        let b = v.get_cached("minecraft:chests/ruined_portal").unwrap();
        // 同一缓存项（Arc 指针相等），且与短名/全 id 解析路径无关
        assert!(Arc::ptr_eq(&a, &b));
        // 缓存命中与直接解析结果一致
        let direct = v.get("minecraft:chests/ruined_portal").unwrap();
        let mut r1 = crate::loot::rng::XoroshiroLootRng::new(7);
        let mut r2 = crate::loot::rng::XoroshiroLootRng::new(7);
        assert_eq!(a.generate(&mut r1, 0.0), direct.generate(&mut r2, 0.0));
    }
}
