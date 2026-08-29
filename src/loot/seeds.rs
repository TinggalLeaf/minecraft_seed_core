//! 结构特化的宝箱种子推导：1:1 翻译 Python 项目 `src/structure_seeds.py`。
//!
//! 关键公式（与 vanilla 1.20.1 对齐）：
//!
//! - 区块 feature seed：
//!   `feature_seed = world_seed + cx*cx*4987142 + cx*5947611 + cz*cz*4392871 + cz*389711`
//! - 三轴 setSeed：
//!   `set_seed_for_block(seed, x, y, z) = x*341873128712 + y*132897987541 + seed + z`
//! - 块位置哈希（vanilla `apa.b`）：
//!   `(x * 3129871) ^ (z * 116129781) ^ (y * -897990906)`，模 2^64。
//! - 默认箱体 RNG：`world_seed ^ block_pos_hash` 后直接喂 Xoroshiro。
//!
//! 每个箱子 `LootTable::generate` 在生成前还会额外消费一次 `next_long()`
//! （对应 Python 端 `predictor.py: rng.next_long()`，模拟结构部件占位
//! 消耗），这是 MC 1.18+ 宝箱生成的实测行为，移植时必须保留等价语义。
//!
//! 36 个 chest id 在 [`SEED_DERIVERS`] 中按 5 种模式分发：
//!   1. `seed_to_chest_rng`        — ruined_portal、16 种 village；
//!   2. `buried_treasure_seed`     — buried_treasure、simple_dungeon；
//!   3. `nether_fortress_chest_seed`— nether_bridge；
//!   4. `desert_pyramid_chest_seed`— desert_pyramid + 22 种其他结构；
//!   5. `stronghold_chest_seed`    — stronghold×3。

use crate::loot::rng::{LootRng, XoroshiroLootRng};

/// 常量（与 Python `rng.py` 一致）。
const MASK_64: u64 = u64::MAX;

/// 区块 feature seed（`defpackage.dij.a`）。
#[inline]
pub fn feature_seed(world_seed: i64, chunk_x: i32, chunk_z: i32) -> u64 {
    let ws = world_seed as u64;
    let cx = chunk_x as i64 as u64;
    let cz = chunk_z as i64 as u64;
    let cx2 = cx.wrapping_mul(cx);
    let cz2 = cz.wrapping_mul(cz);
    let r = ws
        .wrapping_add(cx2.wrapping_mul(4_987_142))
        .wrapping_add(cx.wrapping_mul(5_947_611))
        .wrapping_add(cz2.wrapping_mul(4_392_871))
        .wrapping_add(cz.wrapping_mul(389_711));
    r & MASK_64
}

/// 三轴 setSeed（`defpackage.dij.a` 的 `(long, int, int, int)` 重载）。
#[inline]
pub fn set_seed_for_block(seed: u64, x: i32, y: i32, z: i32) -> u64 {
    let xv = x as i64 as u64;
    let yv = y as i64 as u64;
    let zv = z as i64 as u64;
    xv.wrapping_mul(341_873_128_712)
        .wrapping_add(yv.wrapping_mul(132_897_987_541))
        .wrapping_add(seed)
        .wrapping_add(zv)
        & MASK_64
}

/// 块位置哈希（`defpackage.apa.b`，与 cubiomes 一致）。
#[inline]
pub fn block_pos_hash(x: i32, y: i32, z: i32) -> u64 {
    let xv = x as i64 as u64;
    let yv = y as i64 as u64;
    let zv = z as i64 as u64;
    let h = xv.wrapping_mul(3_129_871) ^ zv.wrapping_mul(116_129_781);
    let h = h ^ yv.wrapping_mul((-897_990_906i64) as u64);
    h & MASK_64
}

/// 默认箱体 RNG 种子：`world_seed ^ block_pos_hash`。
#[inline]
pub fn seed_to_chest_seed(world_seed: i64, x: i32, y: i32, z: i32) -> u64 {
    (world_seed as u64) ^ block_pos_hash(x, y, z)
}

// ---------------------------------------------------------------------------
// 结构特化的派生函数（返回最终种子的 64 位表示）
// ---------------------------------------------------------------------------

/// `seed_to_chest_rng` 直接返回默认 XOR 种子（Python 版在 `chest_seed_for`
/// 中对这种模式再额外消费一次 `next_long`，见模块顶部注释）。
pub fn seed_to_chest_rng_seed(world_seed: i64, x: i32, y: i32, z: i32) -> u64 {
    seed_to_chest_seed(world_seed, x, y, z)
}

/// Buried treasure / simple dungeon：feature_seed 作为种子。
pub fn buried_treasure_seed_fn(world_seed: i64, x: i32, _y: i32, z: i32) -> u64 {
    feature_seed(world_seed, x >> 4, z >> 4)
}

/// Nether fortress / desert pyramid / 大部分结构：先取 feature seed，
/// 再 setSeed(x,y,z)。
pub fn desert_pyramid_chest_seed(world_seed: i64, x: i32, y: i32, z: i32) -> u64 {
    let fs = feature_seed(world_seed, x >> 4, z >> 4);
    set_seed_for_block(fs, x, y, z)
}

/// Simple dungeon：feature_seed 异或 (x*3129871 ^ z*116129781)。
pub fn simple_dungeon_chest_seed(world_seed: i64, x: i32, _y: i32, z: i32) -> u64 {
    let fs = feature_seed(world_seed, x >> 4, z >> 4);
    let xh = (x as i64 as u64).wrapping_mul(3_129_871);
    let zh = (z as i64 as u64).wrapping_mul(116_129_781);
    fs ^ xh ^ zh
}

/// Stronghold：先按 `seed_to_chest_seed` 拿到 xoroshiro，再消耗一次
/// `next_long()` 作为最终种子（结构部件占位 + stronghold 自身的两步种子
/// 推导合并后的对外行为）。
pub fn stronghold_chest_seed(world_seed: i64, x: i32, y: i32, z: i32) -> u64 {
    let s = seed_to_chest_seed(world_seed, x, y, z);
    let mut rng = XoroshiroLootRng::new(s);
    rng.next_long() as u64
}

/// `seed_to_chest_rng` 的完整流程：建立 Xoroshiro 后取首个 long，作为
/// 最终种子（Python 版 `chest_seed_for` 对该分支调用
/// `rng.next_long()`）。
pub fn seed_to_chest_rng_first_long(world_seed: i64, x: i32, y: i32, z: i32) -> u64 {
    let s = seed_to_chest_seed(world_seed, x, y, z);
    let mut rng = XoroshiroLootRng::new(s);
    rng.next_long() as u64
}

// ---------------------------------------------------------------------------
// 表 id → 推导函数 的注册表
// ---------------------------------------------------------------------------

/// 推导函数签名：返回 64 位种子。
pub type SeedFn = fn(i64, i32, i32, i32) -> u64;

/// 36 个 chest id 到对应推导函数的映射（与 Python `SEED_DERIVERS` 等价）。
///
/// `seed_to_chest_rng` 等价于本模块的 [`seed_to_chest_rng_first_long`]：
/// Python `chest_seed_for` 对该分支返回 `Xoroshiro.next_long()`，与直接
/// 返回首个 long 等价。其它分支按字面翻译。
pub const SEED_DERIVERS: &[(&str, SeedFn)] = &[
    // seed_to_chest_rng —— ruined_portal + 16 种 village
    ("minecraft:chests/ruined_portal", seed_to_chest_rng_first_long as SeedFn),
    ("minecraft:chests/village/village_weaponsmith", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_toolsmith", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_armorer", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_cartographer", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_mason", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_shepherd", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_butcher", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_fletcher", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_fisher", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_tannery", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_temple", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_desert_house", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_plains_house", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_taiga_house", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_snowy_house", seed_to_chest_rng_first_long),
    ("minecraft:chests/village/village_savanna_house", seed_to_chest_rng_first_long),
    // buried_treasure / simple_dungeon
    ("minecraft:chests/buried_treasure", buried_treasure_seed_fn),
    ("minecraft:chests/simple_dungeon", simple_dungeon_chest_seed),
    // nether_bridge — nether_fortress 路径
    ("minecraft:chests/nether_bridge", desert_pyramid_chest_seed),
    // desert_pyramid + 22 种结构
    ("minecraft:chests/desert_pyramid", desert_pyramid_chest_seed),
    ("minecraft:chests/jungle_temple", desert_pyramid_chest_seed),
    ("minecraft:chests/jungle_temple_dispenser", desert_pyramid_chest_seed),
    ("minecraft:chests/igloo_chest", desert_pyramid_chest_seed),
    ("minecraft:chests/woodland_mansion", desert_pyramid_chest_seed),
    ("minecraft:chests/underwater_ruin_small", desert_pyramid_chest_seed),
    ("minecraft:chests/underwater_ruin_big", desert_pyramid_chest_seed),
    ("minecraft:chests/shipwreck_map", desert_pyramid_chest_seed),
    ("minecraft:chests/shipwreck_supply", desert_pyramid_chest_seed),
    ("minecraft:chests/shipwreck_treasure", desert_pyramid_chest_seed),
    ("minecraft:chests/abandoned_mineshaft", desert_pyramid_chest_seed),
    ("minecraft:chests/pillager_outpost", desert_pyramid_chest_seed),
    ("minecraft:chests/bastion_treasure", desert_pyramid_chest_seed),
    ("minecraft:chests/bastion_other", desert_pyramid_chest_seed),
    ("minecraft:chests/bastion_bridge", desert_pyramid_chest_seed),
    ("minecraft:chests/bastion_hoglin_stable", desert_pyramid_chest_seed),
    ("minecraft:chests/ancient_city", desert_pyramid_chest_seed),
    ("minecraft:chests/ancient_city_ice_box", desert_pyramid_chest_seed),
    ("minecraft:chests/end_city_treasure", desert_pyramid_chest_seed),
    // stronghold ×3
    ("minecraft:chests/stronghold_library", stronghold_chest_seed),
    ("minecraft:chests/stronghold_crossing", stronghold_chest_seed),
    ("minecraft:chests/stronghold_corridor", stronghold_chest_seed),
];

/// 表 id → 推导函数。未在 [`SEED_DERIVERS`] 中的 chest id 回退到默认
/// `seed_to_chest_rng_first_long`（与 Python `chest_seed_for` 默认值
/// 一致）。
pub fn derive_seed(loot_table_id: &str, world_seed: i64, x: i32, y: i32, z: i32) -> u64 {
    for (id, f) in SEED_DERIVERS {
        if *id == loot_table_id {
            return f(world_seed, x, y, z);
        }
    }
    seed_to_chest_rng_first_long(world_seed, x, y, z)
}

/// 顶层入口：返回可直接用于 `LootTable::generate` 的 xoroshiro。
///
/// 注意：`predictor.py` 会额外消费一次 `next_long()` 模拟结构部件占位
/// 消耗，本函数已把这次消耗包含在内部，调用方拿到 rng 后**不应**再额外
/// `next_long`，直接 `table.generate(&mut rng, 0.0)` 即可。
pub fn chest_rng(world_seed: i64, loot_table_id: &str, x: i32, y: i32, z: i32) -> XoroshiroLootRng {
    let seed = derive_seed(loot_table_id, world_seed, x, y, z);
    let mut rng = XoroshiroLootRng::new(seed);
    // 复刻 `loot_predictor.py: rng.next_long()`：每张表开箱前的占位消耗。
    rng.next_long();
    rng
}

// ---------------------------------------------------------------------------
// 单元测试：seed 推导的代表性 golden
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_pos_hash_matches_python() {
        // Python 参考：((x*3129871) ^ (z*116129781) ^ (y*-897990906)) & MASK_64
        // 手算 (x=100, y=64, z=200):
        let x: i64 = 100 * 3_129_871;
        let z: i64 = 200 * 116_129_781;
        let y: i64 = 64i64.wrapping_mul(-897_990_906);
        let expected = (x ^ z ^ y) as u64;
        assert_eq!(block_pos_hash(100, 64, 200), expected);
    }

    #[test]
    fn seed_to_chest_rng_first_long_matches_python_chest_seed_for() {
        // python:
        //   seed_to_chest_rng(world, 100, 64, 200).next_long()
        // 等价于：Xoroshiro::new(seed).next_long()
        let seed = seed_to_chest_seed(12345, 100, 64, 200);
        let mut xr = XoroshiroLootRng::new(seed);
        let expected = xr.next_long() as u64;
        assert_eq!(seed_to_chest_rng_first_long(12345, 100, 64, 200), expected);
    }

    #[test]
    fn desert_pyramid_seed_known_value() {
        // 与 Python 端：
        //   feature_seed(12345, 100>>4, 200>>4)
        //   set_seed_for_block(...)
        //   => 0x26d4630a90ff
        assert_eq!(derive_seed("minecraft:chests/desert_pyramid", 12345, 100, 64, 200),
                   0x26d4630a90ff);
    }

    #[test]
    fn buried_treasure_seed_known_value() {
        // python: feature_seed(12345, 100>>4, 200>>4) == 0x32cfe3d7
        assert_eq!(derive_seed("minecraft:chests/buried_treasure", 12345, 100, 64, 200),
                   0x32cfe3d7);
    }

    #[test]
    fn stronghold_seed_known_value() {
        // python: seed_to_chest_rng(world, -200, 30, 1500).next_long() == 0xf5e0a9d5887415d8
        assert_eq!(derive_seed("minecraft:chests/stronghold_library", 42, -200, 30, 1500),
                   0xf5e0a9d5887415d8);
    }

    #[test]
    fn end_city_treasure_uses_desert_pyramid_path() {
        assert_eq!(derive_seed("minecraft:chests/end_city_treasure", 12345, 100, 64, 200),
                   0x26d4630a90ff);
    }

    #[test]
    fn bastion_bridge_uses_desert_pyramid_path() {
        assert_eq!(derive_seed("minecraft:chests/bastion_bridge", 12345, 100, 64, 200),
                   0x26d4630a90ff);
    }

    #[test]
    fn fallback_for_unknown_chest_is_seed_to_chest_rng() {
        assert_eq!(
            derive_seed("minecraft:chests/spawn_bonus_chest", 12345, 100, 64, 200),
            seed_to_chest_rng_first_long(12345, 100, 64, 200)
        );
    }
}
