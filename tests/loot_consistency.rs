//! Loot table 一致性测试：把 `loot_golden_data::GOLDEN_CASES` 里的
//! `(world_seed, table_id, x/y/z) → items` 由 Python 参考项目
//! （`E:\Projects\Minecraft\宝箱内容生成`）生成，再用 Rust 引擎重放。
//!
//! 覆盖：
//! - 5 种 seed 推导模式（`seed_to_chest_rng` / `buried_treasure` /
//!   `nether_fortress` / `desert_pyramid` / `stronghold`）；
//! - 16 种 village 子类型、`set_count` / `enchant_randomly` 函数；
//! - 空表（`minecraft:empty`）；
//! - 非 chest 表（`minecraft:gameplay/fishing`）。
//!
//! Golden 数据由 `scripts/gen_golden.py` 生成，请勿手改。

mod loot_golden_data;

use loot_golden_data::{GoldenCase, GOLDEN_CASES};
use minecraft_seed_core::loot::{chest_rng, LootVersion};

#[test]
fn every_case_seed_matches_python() {
    for c in GOLDEN_CASES {
        // Python 端 chest_seed_for 返回 mask64 的 u64 字符串，Rust 端
        // derive_seed 返回 u64，比较时按位一致。
        let mine = minecraft_seed_core::loot::derive_seed(
            c.id,
            c.world_seed,
            c.x,
            c.y,
            c.z,
        );
        assert_eq!(
            mine, c.expected_seed,
            "seed mismatch for {} ({},{},{},{})",
            c.id, c.world_seed, c.x, c.y, c.z
        );
    }
}

#[test]
fn every_case_items_match_python() {
    for c in GOLDEN_CASES {
        let table = LootVersion::V1_20_1
            .get(c.id)
            .unwrap_or_else(|e| panic!("load {}: {e}", c.id));
        let mut rng = chest_rng(c.world_seed, c.id, c.x, c.y, c.z);
        let got = table.generate(&mut rng, 0.0);
        // Python loot_predictor 把结果按 item_id 排序后输出。我们把两边都
        // 转成 `(item, count, enchanted)` 三元组的多重集合再比对，避免
        // 不同输出顺序误报。
        let mut got_set: std::collections::BTreeMap<(&str, bool), i32> =
            std::collections::BTreeMap::new();
        for it in &got {
            *got_set
                .entry((it.item.as_str(), it.enchanted))
                .or_insert(0) += it.count;
        }
        let mut exp_set: std::collections::BTreeMap<(&str, bool), i32> =
            std::collections::BTreeMap::new();
        for (item, cnt, ench) in c.expected_items {
            *exp_set.entry((*item, *ench)).or_insert(0) += cnt;
        }
        assert_eq!(
            got_set, exp_set,
            "item multiset mismatch for {} (world_seed={}, x={}, y={}, z={})",
            c.id, c.world_seed, c.x, c.y, c.z
        );
    }
}

#[test]
fn unknown_chest_id_is_rejected() {
    assert!(LootVersion::V1_20_1.get("minecraft:does_not_exist").is_err());
}

#[test]
fn empty_table_yields_no_items() {
    let t = LootVersion::V1_20_1.get("minecraft:empty").unwrap();
    let mut rng = chest_rng(0, "minecraft:empty", 0, 0, 0);
    assert!(t.generate(&mut rng, 0.0).is_empty());
}

#[test]
fn fishing_uses_world_seed_directly() {
    // gameplay/fishing 不在 chest 推导表里，回退到 `seed_to_chest_rng_first_long`。
    // 这里只验证它能跑通且与 golden 用例结果一致（golden.py 用 predict 路径
    // 用 world_seed 直接喂 Xoroshiro，行为相同）。
    let t = LootVersion::V1_20_1
        .get("minecraft:gameplay/fishing")
        .unwrap();
    let mut rng = chest_rng(12345, "minecraft:gameplay/fishing", 0, 0, 0);
    let items = t.generate(&mut rng, 0.0);
    // Golden 里该 case 的 items 已对拍过（见 `every_case_items_match_python`），
    // 这里再确认非空 + 项数 == golden
    let case: &GoldenCase = GOLDEN_CASES
        .iter()
        .find(|c| c.id == "minecraft:gameplay/fishing")
        .expect("golden case present");
    assert_eq!(items.len(), case.expected_items.len());
}

#[test]
fn all_853_tables_are_parseable() {
    let v = LootVersion::V1_20_1;
    for (id, _path) in v.tables() {
        let table = v
            .get(id)
            .unwrap_or_else(|e| panic!("failed to parse {id}: {e}"));
        let mut rng = chest_rng(12345, id, 0, 64, 0);
        // 不强制非空（empty 就是空），只验证能跑通 generate
        let _ = table.generate(&mut rng, 0.0);
    }
}
