//! 战利品表统计与分析：1:1 翻译 Python `src/loot_stats.py`。
//!
//! 提供两类能力：
//!
//! - **解析解**（[`analyse_table`]）：由 NumberProvider 的精确分布
//!   （constant / uniform / binomial PMF）推导每个 entry 的单次命中概率
//!   `w_i / W` 与期望数量 `E[rolls] × w_i/W × E[stack_size]`。
//! - **蒙特卡洛**（[`simulate`] / [`aggregate_counts`] / [`contains_query`]）：
//!   用 xoroshiro 实际采样，验证解析结果或处理多 pool 重叠等复杂情形。
//!
//! 与 Python 端语义对齐的要点：
//! - `simulate` 非 chest 模式：第 i 个样本的种子是 `world_seed + i`；
//! - `contains_query` 的种子：`world_seed ^ block_pos_hash(x+i, y, z)`
//!   （即 `apa.b` 位置哈希，见 [`crate::loot::seeds::block_pos_hash`]）。

use std::collections::BTreeMap;

use crate::loot::rng::XoroshiroLootRng;
use crate::loot::table::{LootFunction, LootItem, LootTable, NumberProvider};

/// 离散分布：`值 → 概率`。
pub type Pmf = BTreeMap<i32, f64>;

/// 两个离散分布的卷积。
pub fn convolve(a: &Pmf, b: &Pmf) -> Pmf {
    let mut out = Pmf::new();
    for (va, pa) in a {
        for (vb, pb) in b {
            *out.entry(va + vb).or_insert(0.0) += pa * pb;
        }
    }
    out
}

/// 分布的期望。
pub fn expectation(pmf: &Pmf) -> f64 {
    pmf.iter().map(|(v, p)| *v as f64 * p).sum()
}

/// NumberProvider 的精确 PMF（constant / uniform / binomial）。
pub(crate) fn provider_pmf(np: &NumberProvider) -> Pmf {
    match np {
        NumberProvider::Constant(v) => Pmf::from([(*v as i32, 1.0)]),
        NumberProvider::Uniform { lo, hi } => {
            let n = (hi - lo + 1) as f64;
            (*lo..=*hi).map(|v| (v, 1.0 / n)).collect()
        }
        NumberProvider::Binomial { n, p } => {
            // P(k) = C(n,k) p^k (1-p)^(n-k)
            let n = *n as u32;
            (0..=n)
                .map(|k| {
                    let c = binomial_coeff(n, k) as f64;
                    (k as i32, c * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32))
                })
                .collect()
        }
    }
}

/// C(n, k)，精确整数计算（防溢出用 u128 中间值，loot 表里 n 很小）。
fn binomial_coeff(n: u32, k: u32) -> u64 {
    let k = k.min(n - k);
    let mut num: u128 = 1;
    let mut den: u128 = 1;
    for i in 0..k {
        num *= (n - i) as u128;
        den *= (i + 1) as u128;
    }
    (num / den) as u64
}

// ---------------------------------------------------------------------------
// 解析解
// ---------------------------------------------------------------------------

/// 单个 entry 的统计。
#[derive(Clone, Debug)]
pub struct EntryStats {
    pub name: String,
    pub weight: i32,
    /// 单次 roll 命中该 entry 的概率 `w_i / W`。
    pub probability_per_roll: f64,
    /// 每个箱子期望产出数量 `E[rolls] × w_i/W × E[stack_size]`。
    pub expected_count: f64,
    /// 命中时堆叠数量的分布（无 `set_count` 函数时为 `None`，表示恒为 1）。
    pub count_pmf: Option<Pmf>,
}

/// 单个 pool 的统计。
#[derive(Clone, Debug)]
pub struct PoolStats {
    pub index: usize,
    pub rolls_distribution: Pmf,
    pub total_weight: i32,
    pub entries: Vec<EntryStats>,
}

impl PoolStats {
    /// 期望 roll 次数。
    pub fn expected_rolls(&self) -> f64 {
        expectation(&self.rolls_distribution)
    }
}

/// 整张表的统计。
#[derive(Clone, Debug)]
pub struct TableStats {
    pub table_id: String,
    pub pools: Vec<PoolStats>,
}

impl TableStats {
    /// 每箱期望物品总数（各 pool 期望 rolls 之和）。
    pub fn expected_total_items(&self) -> f64 {
        self.pools.iter().map(|p| p.expected_rolls()).sum()
    }

    /// 按物品聚合的期望数量表（item → E[count/chest]）。
    pub fn expected_counts_by_item(&self) -> BTreeMap<String, f64> {
        let mut out: BTreeMap<String, f64> = BTreeMap::new();
        for pool in &self.pools {
            for e in &pool.entries {
                *out.entry(e.name.clone()).or_insert(0.0) += e.expected_count;
            }
        }
        out
    }
}

/// entry 的 `set_count` 分布；无 `set_count` 时返回 `None`（count 恒为 1）。
///
/// 与 Python `_count_pmf` 一致：只查看 entry 顶层 functions，不递归 children
/// （Python 端同样只看一层）。
fn count_pmf_of(entry: &crate::loot::table::LootPoolEntry) -> Option<Pmf> {
    for func in &entry.functions {
        if let LootFunction::SetCount(np) = func {
            return Some(provider_pmf(np));
        }
    }
    None
}

/// 分析单个 pool。
pub(crate) fn analyse_pool(pool: &crate::loot::table::LootPool, index: usize) -> PoolStats {
    let rolls_pmf = provider_pmf(&pool.rolls);
    let eligible: Vec<_> = pool.entries.iter().filter(|e| e.weight > 0).collect();
    let total_w: i32 = eligible.iter().map(|e| e.weight).sum();
    let e_rolls = expectation(&rolls_pmf);
    let entries = eligible
        .iter()
        .map(|e| {
            let prob = e.weight as f64 / total_w as f64;
            let count_pmf = count_pmf_of(e);
            let e_count = count_pmf.as_ref().map(expectation).unwrap_or(1.0);
            EntryStats {
                name: e.name.clone().unwrap_or_default(),
                weight: e.weight,
                probability_per_roll: prob,
                expected_count: e_rolls * prob * e_count,
                count_pmf,
            }
        })
        .collect();
    PoolStats {
        index,
        rolls_distribution: rolls_pmf,
        total_weight: total_w,
        entries,
    }
}

/// 分析整张表（解析解）。
pub fn analyse_table(table_id: &str, table: &LootTable) -> TableStats {
    let pools = table
        .pools_internal()
        .iter()
        .enumerate()
        .map(|(i, p)| analyse_pool(p, i))
        .collect();
    TableStats {
        table_id: table_id.to_string(),
        pools,
    }
}

// ---------------------------------------------------------------------------
// 蒙特卡洛
// ---------------------------------------------------------------------------

/// 采样 `samples` 次，返回每次的物品列表。
///
/// 非 chest 模式（fishing / entities / blocks 等）：第 i 个样本的 RNG 种子
/// 为 `world_seed + i`（与 Python `simulate` 的默认路径一致）。
pub fn simulate(table: &LootTable, world_seed: i64, samples: usize, luck: f64) -> Vec<Vec<LootItem>> {
    let mut out = Vec::with_capacity(samples);
    for i in 0..samples {
        let mut rng = XoroshiroLootRng::new(world_seed.wrapping_add(i as i64) as u64);
        out.push(table.generate(&mut rng, luck));
    }
    out
}

/// [`simulate`] 的多线程版本：结果与单线程逐项一致（样本间无共享状态，
/// 按区间切片后 `std::thread::scope` 并行，最后按原顺序拼接）。
///
/// `threads <= 1` 时退化为单线程。零依赖（仅 std）。
pub fn simulate_par(
    table: &LootTable,
    world_seed: i64,
    samples: usize,
    luck: f64,
    threads: usize,
) -> Vec<Vec<LootItem>> {
    if threads <= 1 || samples < 1024 {
        return simulate(table, world_seed, samples, luck);
    }
    let threads = threads.min(samples);
    let chunk = samples.div_ceil(threads);
    let mut parts: Vec<Vec<Vec<LootItem>>> = Vec::with_capacity(threads);
    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(threads);
        for t in 0..threads {
            let start = t * chunk;
            let end = ((t + 1) * chunk).min(samples);
            if start >= end {
                break;
            }
            handles.push(s.spawn(move || {
                let mut out = Vec::with_capacity(end - start);
                for i in start..end {
                    let mut rng =
                        XoroshiroLootRng::new(world_seed.wrapping_add(i as i64) as u64);
                    out.push(table.generate(&mut rng, luck));
                }
                out
            }));
        }
        for h in handles {
            parts.push(h.join().unwrap());
        }
    });
    let mut out = Vec::with_capacity(samples);
    for part in parts {
        out.extend(part);
    }
    out
}

/// 单个物品的聚合统计。
#[derive(Clone, Debug, Default)]
pub struct ItemAgg {
    /// 至少出现一次的样本数。
    pub appearances: usize,
    /// 总产出件数。
    pub total_count: i64,
    /// `total_count / samples`。
    pub per_roll_avg: f64,
    /// `appearances / samples`。
    pub frequency: f64,
}

/// 对多次采样结果做聚合（item → 统计）。
pub fn aggregate_counts(rolls: &[Vec<LootItem>]) -> BTreeMap<String, ItemAgg> {
    let n = rolls.len();
    let mut stats: BTreeMap<String, ItemAgg> = BTreeMap::new();
    for items in rolls {
        let mut seen = std::collections::HashSet::new();
        for it in items {
            let s = stats.entry(it.item.clone()).or_default();
            s.total_count += it.count as i64;
            if seen.insert(it.item.clone()) {
                s.appearances += 1;
            }
        }
    }
    for s in stats.values_mut() {
        if n > 0 {
            s.per_roll_avg = s.total_count as f64 / n as f64;
            s.frequency = s.appearances as f64 / n as f64;
        }
    }
    stats
}

/// `contains_query` 的结果。
#[derive(Clone, Debug, Default)]
pub struct ContainsResult {
    pub item: String,
    pub samples: usize,
    pub appearances: usize,
    pub total_count: i64,
    pub max_in_one_chest: i32,
    pub frequency: f64,
    pub avg_when_seen: f64,
}

/// 某物品在该表中出现的概率查询。
///
/// 第 i 个样本的种子为
/// `world_seed ^ block_pos_hash(block_x + i, block_y, block_z)`，
/// 与 Python `contains_query` 逐位一致。
pub fn contains_query(
    table: &LootTable,
    item: &str,
    world_seed: i64,
    samples: usize,
    block_x: i32,
    block_y: i32,
    block_z: i32,
) -> ContainsResult {
    let item_norm = if item.starts_with("minecraft:") {
        item.to_string()
    } else {
        format!("minecraft:{item}")
    };
    let mut out = ContainsResult {
        item: item_norm.clone(),
        samples,
        ..Default::default()
    };
    for i in 0..samples {
        let seed = (world_seed as u64)
            ^ crate::loot::seeds::block_pos_hash(block_x.wrapping_add(i as i32), block_y, block_z);
        let mut rng = XoroshiroLootRng::new(seed);
        let stacks = table.generate(&mut rng, 0.0);
        let cnt: i32 = stacks
            .iter()
            .filter(|s| s.item == item_norm)
            .map(|s| s.count)
            .sum();
        if cnt > 0 {
            out.appearances += 1;
            out.total_count += cnt as i64;
            out.max_in_one_chest = out.max_in_one_chest.max(cnt);
        }
    }
    if samples > 0 {
        out.frequency = out.appearances as f64 / samples as f64;
    }
    if out.appearances > 0 {
        out.avg_when_seen = out.total_count as f64 / out.appearances as f64;
    }
    out
}

/// [`contains_query`] 的多线程版本：结果与单线程一致（按样本区间切片
/// 并行后合并计数）。`threads <= 1` 时退化为单线程。
pub fn contains_query_par(
    table: &LootTable,
    item: &str,
    world_seed: i64,
    samples: usize,
    block_x: i32,
    block_y: i32,
    block_z: i32,
    threads: usize,
) -> ContainsResult {
    if threads <= 1 || samples < 1024 {
        return contains_query(table, item, world_seed, samples, block_x, block_y, block_z);
    }
    let threads = threads.min(samples);
    let chunk = samples.div_ceil(threads);
    let mut merged = ContainsResult {
        item: if item.starts_with("minecraft:") {
            item.to_string()
        } else {
            format!("minecraft:{item}")
        },
        samples,
        ..Default::default()
    };
    std::thread::scope(|s| {
        let mut handles = Vec::with_capacity(threads);
        for t in 0..threads {
            let start = t * chunk;
            let end = ((t + 1) * chunk).min(samples);
            if start >= end {
                break;
            }
            handles.push(s.spawn(move || {
                contains_query(
                    table,
                    item,
                    world_seed,
                    end - start,
                    block_x.wrapping_add(start as i32),
                    block_y,
                    block_z,
                )
            }));
        }
        for h in handles {
            let part = h.join().unwrap();
            merged.appearances += part.appearances;
            merged.total_count += part.total_count;
            merged.max_in_one_chest = merged.max_in_one_chest.max(part.max_in_one_chest);
        }
    });
    if samples > 0 {
        merged.frequency = merged.appearances as f64 / samples as f64;
    }
    if merged.appearances > 0 {
        merged.avg_when_seen = merged.total_count as f64 / merged.appearances as f64;
    }
    merged
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_pmf_is_flat() {
        let pmf = provider_pmf(&NumberProvider::Uniform { lo: 2, hi: 5 });
        assert_eq!(pmf.len(), 4);
        for p in pmf.values() {
            assert!((p - 0.25).abs() < 1e-12);
        }
    }

    #[test]
    fn binomial_pmf_matches_formula() {
        // Binomial(2, 0.5) = {0: 0.25, 1: 0.5, 2: 0.25}
        let pmf = provider_pmf(&NumberProvider::Binomial { n: 2, p: 0.5 });
        assert!((pmf[&0] - 0.25).abs() < 1e-12);
        assert!((pmf[&1] - 0.5).abs() < 1e-12);
        assert!((pmf[&2] - 0.25).abs() < 1e-12);
    }

    #[test]
    fn convolve_adds_supports() {
        let a = Pmf::from([(1, 0.5), (2, 0.5)]);
        let b = Pmf::from([(1, 1.0)]);
        let c = convolve(&a, &b);
        assert_eq!(c, Pmf::from([(2, 0.5), (3, 0.5)]));
    }

    #[test]
    fn analyse_ruined_portal_totals() {
        // ruined_portal：单 pool，uniform rolls 4..8（E=6），总权重 398。
        let v = crate::loot::registry::LootVersion::V1_20_1;
        let table = v.get("minecraft:chests/ruined_portal").unwrap();
        let stats = analyse_table("minecraft:chests/ruined_portal", &table);
        assert_eq!(stats.pools.len(), 1);
        let pool = &stats.pools[0];
        assert!((pool.expected_rolls() - 6.0).abs() < 1e-9);
        assert_eq!(pool.total_weight, 398);
        // 权重和概率的归一性
        let p_sum: f64 = pool.entries.iter().map(|e| e.probability_per_roll).sum();
        assert!((p_sum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn simulate_and_aggregate_are_consistent() {
        let v = crate::loot::registry::LootVersion::V1_20_1;
        let table = v.get("minecraft:entities/zombie").unwrap();
        let rolls = simulate(&table, 12345, 100, 0.0);
        assert_eq!(rolls.len(), 100);
        let agg = aggregate_counts(&rolls);
        // zombie 掉落 rotten_flesh（条件恒通过）
        assert!(agg.contains_key("minecraft:rotten_flesh"));
        let rf = &agg["minecraft:rotten_flesh"];
        assert!(rf.frequency > 0.0 && rf.frequency <= 1.0);
    }

    #[test]
    fn contains_query_finds_common_item() {
        let v = crate::loot::registry::LootVersion::V1_20_1;
        let table = v.get("minecraft:chests/simple_dungeon").unwrap();
        // simple_dungeon 几乎所有样本都含某种常见物品；用 wheat（权重 20/130）
        let r = contains_query(&table, "wheat", 12345, 1000, 0, 64, 0);
        assert!(r.appearances > 0);
        assert!(r.frequency > 0.05, "frequency={}", r.frequency);
    }

    #[test]
    fn simulate_par_matches_single_threaded() {
        let v = crate::loot::registry::LootVersion::V1_20_1;
        let table = v.get("minecraft:chests/desert_pyramid").unwrap();
        let single = simulate(&table, 999, 5000, 0.0);
        let par = simulate_par(&table, 999, 5000, 0.0, 4);
        assert_eq!(single, par);
    }

    #[test]
    fn contains_query_par_matches_single_threaded() {
        let v = crate::loot::registry::LootVersion::V1_20_1;
        let table = v.get("minecraft:chests/simple_dungeon").unwrap();
        let single = contains_query(&table, "wheat", 12345, 5000, 0, 64, 0);
        let par = contains_query_par(&table, "wheat", 12345, 5000, 0, 64, 0, 4);
        assert_eq!(single.appearances, par.appearances);
        assert_eq!(single.total_count, par.total_count);
        assert_eq!(single.max_in_one_chest, par.max_in_one_chest);
        assert_eq!(single.frequency, par.frequency);
    }
}
