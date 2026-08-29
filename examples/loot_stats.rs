//! 战利品表统计分析示例：解析概率 + 蒙特卡洛 + 单物品命中率查询。
//!
//! 对应 Python 参考项目 `loot_predictor.py` 的 --analyze / --bulk /
//! --contains / --compare 模式。
//!
//! 运行：`cargo run --example loot_stats`
use minecraft_seed_core::loot::{self, LootVersion};

fn main() {
    let v = LootVersion::V1_20_1;
    let seed: i64 = 12345;

    // ---- 版本与分类总览（按版本号分类的数据） ----
    println!("=== 已注册战利品表版本 ===");
    for ver in LootVersion::ALL {
        println!(
            "{ver:?}: {} 张表（chests={} entities={} gameplay={} archaeology={} blocks={}）",
            ver.tables().len(),
            ver.category("chests").len(),
            ver.category("entities").len(),
            ver.category("gameplay").len(),
            ver.category("archaeology").len(),
            ver.category("blocks").len(),
        );
    }

    // ---- 解析分析（--analyze）：废弃传送门 ----
    let stats = loot::analyze(v, "ruined_portal").unwrap();
    println!("\n=== 解析分析：{} ===", stats.table_id);
    for pool in &stats.pools {
        println!(
            "  pool {}: E[rolls]={:.2} 总权重={}",
            pool.index,
            pool.expected_rolls(),
            pool.total_weight
        );
    }
    let mut items: Vec<_> = stats.expected_counts_by_item().into_iter().collect();
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("  期望数量前 5：");
    for (name, e) in items.iter().take(5) {
        println!("    {name:<40} E={e:.4}/箱");
    }

    // ---- 蒙特卡洛（--bulk）：沙漠神殿 10000 次采样 ----
    let table = v.get("minecraft:chests/desert_pyramid").unwrap();
    let rolls = loot::simulate(&table, seed, 10_000, 0.0);
    let agg = loot::aggregate_counts(&rolls);
    println!("\n=== 蒙特卡洛：desert_pyramid ×10000 ===");
    let mut rows: Vec<_> = agg.iter().collect();
    rows.sort_by(|a, b| b.1.frequency.partial_cmp(&a.1.frequency).unwrap());
    for (name, s) in rows.iter().take(6) {
        println!(
            "  {name:<40} 频率={:>6.2}%  平均={:.3}/箱",
            s.frequency * 100.0,
            s.per_roll_avg
        );
    }

    // ---- 单物品查询（--contains）：附魔金苹果 ----
    println!("\n=== 附魔金苹果出现率（10000 样本） ===");
    for chest in ["ruined_portal", "dungeon", "desert_pyramid", "bastion_treasure"] {
        let r = loot::contains_probability(v, chest, "enchanted_golden_apple", seed, 10_000)
            .unwrap();
        println!(
            "  {chest:<20} {:>6.2}%  最多 {}/箱",
            r.frequency * 100.0,
            r.max_in_one_chest
        );
    }

    // ---- 非 chest 表：方块掉落（时运）与实体掉落 ----
    println!("\n=== 非 chest 表 ===");
    let drops = loot::predict_table(v, "minecraft:entities/zombie", seed, 0).unwrap();
    println!("zombie 掉落（seed={seed}）：{drops:?}");
    let ore = v.get("minecraft:blocks/diamond_ore").unwrap();
    let mut rng = loot::XoroshiroLootRng::new(seed as u64);
    let no_fortune = ore.generate(&mut rng, 0.0);
    let mut rng = loot::XoroshiroLootRng::new(seed as u64);
    let fortune3 = ore.generate_ctx(&mut rng, 0.0, 3);
    println!("diamond_ore：无时运 {no_fortune:?}，时运 III {fortune3:?}");
}
