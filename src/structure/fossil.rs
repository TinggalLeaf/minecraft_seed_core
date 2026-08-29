//! 化石（Fossil）位置计算：mcseedmap/chunkbase 前端 JS 的逐位复刻。
//!
//! 化石**不是** scattered feature：没有 region 散布，而是对主世界每个
//! 16×16 区块做两次独立判定（salt 30000 与 30001，各 1/64 概率，合计
//! 每区块约 1/32）。vanilla 中化石 1.20 加入，只出现在沙漠/沼泽/
//! 红树林沼泽（生物群系过滤由调用方用
//! [`crate::structure::is_viable_feature_biome`] 完成；网站前端只算
//! 位置，不过滤群系）。
//!
//! 算法（mcseedmap `chunk-874.js` 已证实）：
//!
//! ```text
//! u = (x*a + z*b) ^ world_seed          （x,z 为区块起点的方块坐标）
//!     其中 a,b 为以 world_seed 播种的 RNG 的两个 long，各强制最低位置 1
//!     - mc >= 1.21.1：xoroshiro128+（nextLongJ × 2）
//!     - mc <  1.21.1：Java LCG（nextLong × 2）
//! 对每个 salt ∈ {30000, 30001}：
//!     r = RNG(u + salt)
//!     if r.nextInt(64) == 0:
//!         pos = (x + r.nextInt(16), z + r.nextInt(16))
//! ```
//!
//! 即 chunk 种子派生与 [`get_population_seed`](super::region::get_population_seed)
//! 同族，但 RNG 切换阈值是 **1.21.1**（网站的 biome-layer ≥ 28），而非
//! `get_population_seed` 的 1.18——两处行为不同是网站实现的事实，勿合并。

use crate::rng::{JavaRandom, Xoroshiro};
use crate::version::McVersion;
use crate::structure::region::Pos;

/// 化石判定用的两个 salt（vanilla 的 fossil_upper / fossil_lower 两次尝试）。
pub const FOSSIL_SALTS: [u64; 2] = [30000, 30001];

/// 每 salt 的命中概率：`nextInt(64) == 0`。
pub const FOSSIL_RARITY: u32 = 64;

/// 化石的区块种子派生（mcseedmap `ed()`）。
///
/// `x` / `z` 为方块坐标（区块起点 = 区块坐标 × 16）。
///
/// 注意：网站 JS 的 `en.nextLong()` 用 `(a << 32) | b`（**无符号**拼接），
/// 与 Java `((long)a << 32) + b` 在低 32 位最高位置 1 时结果不同。
/// 为与网站逐位一致，这里复刻 JS 语义（`next(32)` 取无符号值后按位或）。
fn fossil_chunk_seed(mc: McVersion, world_seed: u64, x: i32, z: i32) -> u64 {
    let (a, b) = if mc >= McVersion::V1_21_1 {
        let mut xr = Xoroshiro::new(world_seed);
        (xr.next_long_j(), xr.next_long_j())
    } else {
        let mut r = JavaRandom::new(world_seed as i64);
        let next_long_js = |r: &mut JavaRandom| -> u64 {
            let hi = r.next(32) as u32 as u64;
            let lo = r.next(32) as u32 as u64;
            (hi << 32) | lo
        };
        (next_long_js(&mut r), next_long_js(&mut r))
    };
    let a = a | 1;
    let b = b | 1;
    (x as i64 as u64)
        .wrapping_mul(a)
        .wrapping_add((z as i64 as u64).wrapping_mul(b))
        ^ world_seed
}

/// 判定单个区块内的化石位置（0–2 个）。
///
/// - `chunk_x` / `chunk_z`：区块坐标。
/// - 返回每次命中的方块级 `(x, z)`（chunk 内偏移 0–15）。
/// - `mc < V1_20` 恒为空（vanilla 1.20 才加入化石；网站 UI 虽对 1.12+
///   显示该图层，本库按 vanilla 版本门控）。
pub fn get_fossil_positions(
    mc: McVersion,
    world_seed: u64,
    chunk_x: i32,
    chunk_z: i32,
) -> Vec<Pos> {
    let mut out = Vec::new();
    if mc < McVersion::V1_20 {
        return out;
    }
    let (bx, bz) = (chunk_x * 16, chunk_z * 16);
    let u = fossil_chunk_seed(mc, world_seed, bx, bz);
    for salt in FOSSIL_SALTS {
        let s = u.wrapping_add(salt);
        if mc >= McVersion::V1_21_1 {
            let mut xr = Xoroshiro::new(s);
            if xr.next_int_j(FOSSIL_RARITY) == 0 {
                out.push(Pos {
                    x: bx + xr.next_int_j(16),
                    z: bz + xr.next_int_j(16),
                });
            }
        } else {
            let mut r = JavaRandom::new(s as i64);
            if r.next_int_bound(FOSSIL_RARITY as i32) == 0 {
                out.push(Pos {
                    x: bx + r.next_int_bound(16),
                    z: bz + r.next_int_bound(16),
                });
            }
        }
    }
    out
}

/// 扫描一个区块矩形范围内的全部化石位置。
///
/// 范围以区块坐标给出，含两端（与网站扫描语义一致）。
pub fn scan_fossils(
    mc: McVersion,
    world_seed: u64,
    min_cx: i32,
    min_cz: i32,
    max_cx: i32,
    max_cz: i32,
) -> Vec<Pos> {
    let mut out = Vec::new();
    // 与网站 JS 相同的遍历顺序：外层 x、内层 z
    for cx in min_cx..=max_cx {
        for cz in min_cz..=max_cz {
            out.extend(get_fossil_positions(mc, world_seed, cx, cz));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_fossils_before_1_20() {
        for mc in [McVersion::V1_13, McVersion::V1_16_1, McVersion::V1_19_2] {
            assert!(get_fossil_positions(mc, 12345, 0, 0).is_empty());
        }
    }

    #[test]
    fn fossil_density_matches_one_in_32() {
        // 1000×1000 区块 × 2 salt × 1/64 ≈ 31250 ± 噪声；宽松区间校验
        let n = scan_fossils(McVersion::V1_21, 12345, -500, -500, 499, 499).len();
        let expected = 1_000_000.0 * 2.0 / 64.0;
        assert!(
            (n as f64 - expected).abs() < expected * 0.05,
            "fossil count {n} far from expected {expected}"
        );
    }

    #[test]
    fn deterministic_per_chunk() {
        let a = get_fossil_positions(McVersion::V1_20, 999, 3, -7);
        let b = get_fossil_positions(McVersion::V1_20, 999, 3, -7);
        assert_eq!(a, b);
    }

    #[test]
    fn version_switch_changes_rng_path() {
        // 1.20.6（JavaRandom 路径）与 1.21.1（xoroshiro 路径）对同一
        // 区块应给出不同的判定序列（统计上几乎必然不同）。
        let a = scan_fossils(McVersion::V1_20, 777, -50, -50, 49, 49);
        let b = scan_fossils(McVersion::V1_21_1, 777, -50, -50, 49, 49);
        assert_ne!(a, b);
    }
}
