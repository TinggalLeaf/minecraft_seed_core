//! 四连底座（quad-base）高速搜索：移植 cubiomes `quadbase.c` / `quadbase.h`。
//!
//! ## 原理（译自 C 头文件注释）
//!
//! 「四连结构」指相邻四个 region —— `(0,0)`、`(0,1)`、`(1,0)`、`(1,1)` —— 中
//! 的四个同种结构（典型如沼泽小屋、海底神殿）彼此距离足够近，可以被一个半径
//! 128 方块的球面包住，从而让一个 AFK 点同时覆盖四座刷怪结构（四连女巫小屋
//! 刷怪塔的核心）。由于只关心相对位置，只需检查原点附近这四个 region，再用
//! [`super::move_structure`] 把底座平移到目标位置。
//!
//! 决定 region 内区块位置的 PRNG 对 48 位数做模运算，对「近乎完美」的结构
//! 位置会在种子的低比特上产生限制：低 20 位只有若干种取值（见
//! [`LOW20_QUAD_IDEAL`] 等表），自由位从 48 降到 28，可以暴力枚举出全部
//! 四连结构候选种子。每个候选种子描述了整组可能的四连小屋（region 平移与
//! 高 16 位是自由度数）。
//!
//! ## 与 C 的差异
//!
//! - C 的表以 `0` 结尾、遍历时遇 0 停止；本模块用切片，**不含**结尾的 0。
//! - `searchAll48` 的文件输出 / 断点续传（`.partN` 进度文件）未移植；
//!   [`search_all48`] 只返回内存中的种子列表。其余枚举顺序与 C 逐位一致。
//! - `isQuadBase` 对未支持的结构类型在 C 中 `exit(-1)`，这里 `panic!`。
//! - 浮点返回值与 C 的 `float` 逐位一致（`sqrtf` ↔ `f32::sqrt`，均为 IEEE
//!   正确舍入；C 侧以 `-ffp-contract=off` 编译，无 FMA 融合）。

use std::sync::atomic::{AtomicBool, Ordering};

use super::config::StructureConfig;
use super::region::{Pos, move_structure};
use crate::rng::mul_inv;

// ============================================================================
// 低 20 位星座表（C 静态表，去掉结尾的 0）
// ============================================================================

/// `low20QuadIdeal`：低 20 位，仅保留最佳星座（使用时需减去结构 salt）。
pub const LOW20_QUAD_IDEAL: &[u64] = &[0x43f18, 0xc751a, 0xf520a];

/// `low20QuadClassic`：低 20 位，经典四连结构星座。
pub const LOW20_QUAD_CLASSIC: &[u64] = &[0x43f18, 0x79a0a, 0xc751a, 0xf520a];

/// `low20QuadHutNormal`：结构尺寸 (7+1, 7+43+1, 9+1) 成立的任意有效星座
/// （带摔落伤害通道的四连女巫塔，但可能要求精确的玩家站位）。
pub const LOW20_QUAD_HUT_NORMAL: &[u64] = &[
    0x43f18, 0x65118, 0x75618, 0x79a0a, 0x89718, 0x9371a, 0xa5a08, 0xb5e18, 0xc751a, 0xf520a,
];

/// `low20QuadHutBarely`：结构尺寸 (7+1, 7+1, 9+1) 成立的任意有效星座
/// （无摔落通道的四连女巫塔）。
pub const LOW20_QUAD_HUT_BARELY: &[u64] = &[
    0x1272d, 0x17908, 0x367b9, 0x43f18, 0x487c9, 0x487ce, 0x50aa7, 0x647b5, 0x65118, 0x75618,
    0x79a0a, 0x89718, 0x9371a, 0x967ec, 0xa3d0a, 0xa5918, 0xa591d, 0xa5a08, 0xb5e18, 0xc6749,
    0xc6d9a, 0xc751a, 0xd7108, 0xd717a, 0xe2739, 0xe9918, 0xee1c4, 0xf520a,
];

/// 星座分类（C `enum { CST_NONE, CST_IDEAL, CST_CLASSIC, CST_NORMAL, CST_BARELY }`）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuadHutCst {
    /// 不属于任何已知星座。
    None,
    /// 最佳星座。
    Ideal,
    /// 经典星座。
    Classic,
    /// 带摔落通道的四连女巫塔可用。
    Normal,
    /// 无摔落通道的四连女巫塔可用。
    Barely,
}

/// `getQuadHutCst`：对低 20 位取值分类。
///
/// 与 C 一致按 Ideal → Classic → Normal → Barely 顺序查表，首个命中生效
/// （Ideal/Classic 的条目也出现在更宽的表中）。
pub fn get_quad_hut_cst(low20: u64) -> QuadHutCst {
    for &cst in LOW20_QUAD_IDEAL {
        if cst == low20 {
            return QuadHutCst::Ideal;
        }
    }
    for &cst in LOW20_QUAD_CLASSIC {
        if cst == low20 {
            return QuadHutCst::Classic;
        }
    }
    for &cst in LOW20_QUAD_HUT_NORMAL {
        if cst == low20 {
            return QuadHutCst::Normal;
        }
    }
    for &cst in LOW20_QUAD_HUT_BARELY {
        if cst == low20 {
            return QuadHutCst::Barely;
        }
    }
    QuadHutCst::None
}

// ============================================================================
// 多结构底座判定
// ============================================================================

/// C `JAVA_NEXT_INT24` 宏：一次 LCG 推进后取 `nextInt(24)`（魔数除法，
/// 非 2 次幂范围无拒绝采样，与 `finders.h` 内联快路径一致）。
#[inline(always)]
fn next_int24(s: &mut u64) -> i32 {
    let a = 0x5deece66du64.wrapping_mul(*s).wrapping_add(11) & ((1u64 << 48) - 1);
    *s = a;
    let a = ((a as i64) >> 17) as u64;
    let c = ((0xaaaaaaabu64.wrapping_mul(a) as i64) >> 36) as u64;
    (a as i32).wrapping_sub(((c << 3) as i32).wrapping_mul(3))
}

/// C 静态函数 `getEnclosingRadius`：暴力搜索四个结构的最小包球半径。
///
/// 坐标为 region 内区块偏移；`ax,ay,az` 为结构尺寸，`reg` 为 region 边长
/// （区块），`gap` 为中心点搜索内矩形相对结构包围盒的收缩量（传 radius）。
#[allow(clippy::too_many_arguments)]
fn get_enclosing_radius(
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    x3: i32,
    z3: i32,
    ax: i32,
    ay: i32,
    az: i32,
    reg: i32,
    gap: i32,
) -> f32 {
    // 区块坐标转方块坐标
    let x0 = x0 << 4;
    let z0 = z0 << 4;
    let x1 = ((reg + x1) << 4) + ax;
    let z1 = ((reg + z1) << 4) + az;
    let x2 = ((reg + x2) << 4) + ax;
    let z2 = z2 << 4;
    let x3 = x3 << 4;
    let z3 = ((reg + z3) << 4) + az;

    let mut sqrad = 0x7fffffff;

    // 构造包含中心点的内矩形
    let cbx0 = (if x1 > x2 { x1 } else { x2 }) - gap;
    let cbz0 = (if z1 > z3 { z1 } else { z3 }) - gap;
    let cbx1 = (if x0 < x3 { x0 } else { x3 }) + gap;
    let cbz1 = (if z0 < z2 { z0 } else { z2 }) + gap;

    // 暴力枚举理想中心
    let mut z = cbz0;
    while z <= cbz1 {
        let mut x = cbx0;
        while x <= cbx1 {
            let mut sq = 0;
            let mut s = (x - x0) * (x - x0) + (z - z0) * (z - z0);
            if s > sq {
                sq = s;
            }
            s = (x - x1) * (x - x1) + (z - z1) * (z - z1);
            if s > sq {
                sq = s;
            }
            s = (x - x2) * (x - x2) + (z - z2) * (z - z2);
            if s > sq {
                sq = s;
            }
            s = (x - x3) * (x - x3) + (z - z3) * (z - z3);
            if s > sq {
                sq = s;
            }
            if sq < sqrad {
                sqrad = sq;
            }
            x += 1;
        }
        z += 1;
    }

    if sqrad < 0x7fffffff {
        (sqrad as f32 + (ay * ay) as f32 / 4.0).sqrt()
    } else {
        0xffff as f32
    }
}

/// `isQuadBase`：判定种子低 48 位是否构成四连底座。
///
/// 是下列具体筛选函数的包装：返回 0 表示不是四连底座；否则返回包球半径，
/// 可作为底座质量度量（越小越好）。底座可用 [`super::move_structure`]
/// 平移到其他位置；种子高 16 位不影响结构位置，可自由选择。
///
/// 未支持的结构类型（如林地府邸、要塞等）会 panic（C 中为 `exit(-1)`）。
pub fn is_quad_base(sconf: &StructureConfig, seed: u64, radius: i32) -> f32 {
    use super::config::StructureType::*;
    match sconf.struct_type {
        SwampHut => {
            if radius == 128 {
                is_quad_base_feature24(sconf, seed, 7 + 1, 7 + 1, 9 + 1)
            } else {
                is_quad_base_feature(sconf, seed, 7 + 1, 7 + 1, 9 + 1, radius)
            }
        }
        DesertPyramid | JungleTemple | Igloo | Village => {
            // 这些结构不刷怪，意义不大
            if radius == 128 {
                is_quad_base_feature24(sconf, seed, 0, 0, 0)
            } else {
                is_quad_base_feature(sconf, seed, 0, 0, 0, radius)
            }
        }
        Outpost => {
            // 前哨站还需 1/5 的额外 PRNG 判定且附近不能有村庄；由于其生成点
            // 恒间隔 8 区块，不存在完美的四连前哨站（C 注释：瞭望塔可能偏移
            // 一两个区块，TODO 待研究）
            is_quad_base_feature(sconf, seed, 72, 54, 72, radius)
        }
        Monument => is_quad_base_large(sconf, seed, 58, 23, 58, radius),
        OceanRuin | Shipwreck | RuinedPortal => {
            is_quad_base_feature(sconf, seed, 0, 0, 0, radius)
        }
        other => panic!("isQuadBase: not implemented for structure type {other:?}"),
    }
}

/// `isQuadBaseFeature24`：region=32、chunkRange=24、radius=128 的优化变体。
///
/// `ax,ay,az` 为要求的结构尺寸（含不允许消失的附加空间，如摔落通道）。
/// 返回四个结构内部最远方块到包球中心的实际半径；不满足要求时返回 0。
#[inline]
pub fn is_quad_base_feature24(sconf: &StructureConfig, seed: u64, ax: i32, ay: i32, az: i32) -> f32 {
    const K: u64 = 0x5deece66d;
    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let s00 = seed;
    let s11 = 341873128712u64
        .wrapping_add(132897987541)
        .wrapping_add(seed);

    // 检查对角两个 quadrant 的结构是否足够接近
    let mut s00 = s00 ^ K;
    let x0 = next_int24(&mut s00);
    if x0 < 20 {
        return 0.0;
    }
    let z0 = next_int24(&mut s00);
    if z0 < 20 {
        return 0.0;
    }

    let mut s11 = s11 ^ K;
    let x1 = next_int24(&mut s11);
    if x1 > x0 - 20 {
        return 0.0;
    }
    let z1 = next_int24(&mut s11);
    if z1 > z0 - 20 {
        return 0.0;
    }

    let x = x1 + 32 - x0;
    let z = z1 + 32 - z0;
    if x * x + z * z > 255 {
        return 0.0;
    }

    let s01 = 341873128712u64.wrapping_add(seed);
    let s10 = 132897987541u64.wrapping_add(seed);

    let mut s01 = s01 ^ K;
    let x2 = next_int24(&mut s01);
    if x2 >= 4 {
        return 0.0;
    }
    let z2 = next_int24(&mut s01);
    if z2 < 20 {
        return 0.0;
    }

    let mut s10 = s10 ^ K;
    let x3 = next_int24(&mut s10);
    if x3 < 20 {
        return 0.0;
    }
    let z3 = next_int24(&mut s10);
    if z3 >= 4 {
        return 0.0;
    }

    let x = x2 + 32 - x3;
    let z = z3 + 32 - z2;
    if x * x + z * z > 255 {
        return 0.0;
    }

    // 只有约 1 亿分之一的种子能到这里：判定是否存在以某个方块为中心、
    // 覆盖全部四个结构的球
    let sqrad = get_enclosing_radius(x0, z0, x1, z1, x2, z2, x3, z3, ax, ay, az, 32, 128);
    if sqrad < 128.0 { sqrad } else { 0.0 }
}

/// `isQuadBaseFeature24Classic`：只找经典星座的变体。
#[inline]
pub fn is_quad_base_feature24_classic(sconf: &StructureConfig, seed: u64) -> f32 {
    const K: u64 = 0x5deece66d;
    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let s00 = seed;
    let s11 = 341873128712u64
        .wrapping_add(132897987541)
        .wrapping_add(seed);

    // 检查对角两个 quadrant 的结构是否足够接近
    let mut s00 = s00 ^ K;
    let p = next_int24(&mut s00);
    if p < 22 {
        return 0.0;
    }
    let p = next_int24(&mut s00);
    if p < 22 {
        return 0.0;
    }

    let mut s11 = s11 ^ K;
    let p = next_int24(&mut s11);
    if p > 1 {
        return 0.0;
    }
    let p = next_int24(&mut s11);
    if p > 1 {
        return 0.0;
    }

    let s01 = 341873128712u64.wrapping_add(seed);
    let s10 = 132897987541u64.wrapping_add(seed);

    let mut s01 = s01 ^ K;
    let p = next_int24(&mut s01);
    if p > 1 {
        return 0.0;
    }
    let p = next_int24(&mut s01);
    if p < 22 {
        return 0.0;
    }

    let mut s10 = s10 ^ K;
    let p = next_int24(&mut s10);
    if p < 22 {
        return 0.0;
    }
    let p = next_int24(&mut s10);
    if p > 1 {
        return 0.0;
    }

    1.0 // 实际应返回 122.781311 或 127.887650 之一
}

/// `isQuadBaseFeature`：小型结构的一般形式（chunkRange 非 2 次幂）。
#[inline]
pub fn is_quad_base_feature(
    sconf: &StructureConfig,
    seed: u64,
    ax: i32,
    ay: i32,
    az: i32,
    radius: i32,
) -> f32 {
    const M: u64 = (1u64 << 48) - 1;
    const K: u64 = 0x5deece66d;
    const B: u64 = 0xb;
    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let s00 = seed;
    let s11 = 341873128712u64
        .wrapping_add(132897987541)
        .wrapping_add(seed);

    let r = sconf.region_size;
    let c = sconf.chunk_range;
    let cd = radius / 8;
    let rm = r - ((cd * cd - (r - c + 1) * (r - c + 1)) as f32).sqrt() as i32;

    let mut s = s00 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let x0 = ((s >> 17) as i32) % c;
    if x0 <= rm {
        return 0.0;
    }
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let z0 = ((s >> 17) as i32) % c;
    if z0 <= rm {
        return 0.0;
    }

    s = s11 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let x1 = ((s >> 17) as i32) % c;
    if x1 >= x0 - rm {
        return 0.0;
    }
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let z1 = ((s >> 17) as i32) % c;
    if z1 >= z0 - rm {
        return 0.0;
    }

    // 检查对角两个 quadrant 的结构是否足够接近
    let x = x1 + r - x0;
    let z = z1 + r - z0;
    if x * x + z * z > cd * cd {
        return 0.0;
    }

    let s01 = 341873128712u64.wrapping_add(seed);
    let s10 = 132897987541u64.wrapping_add(seed);

    s = s01 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let x2 = ((s >> 17) as i32) % c;
    if x2 >= c - rm {
        return 0.0;
    }
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let z2 = ((s >> 17) as i32) % c;
    if z2 <= rm {
        return 0.0;
    }

    s = s10 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let x3 = ((s >> 17) as i32) % c;
    if x3 <= rm {
        return 0.0;
    }
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let z3 = ((s >> 17) as i32) % c;
    if z3 >= c - rm {
        return 0.0;
    }

    let x = x2 + r - x3;
    let z = z3 + r - z2;
    if x * x + z * z > cd * cd {
        return 0.0;
    }

    let sqrad = get_enclosing_radius(
        x0,
        z0,
        x1,
        z1,
        x2,
        z2,
        x3,
        z3,
        ax,
        ay,
        az,
        sconf.region_size,
        radius,
    );
    if sqrad < radius as f32 { sqrad } else { 0.0 }
}

/// `isQuadBaseLarge`：大型结构（海底神殿等，三角分布定位）的一般形式。
///
/// 好的四连海底神殿底座极其稀有，且无法用低 20 位方法缩写，搜索耗时
/// 长得多。
#[inline]
pub fn is_quad_base_large(
    sconf: &StructureConfig,
    seed: u64,
    ax: i32,
    ay: i32,
    az: i32,
    radius: i32,
) -> f32 {
    const M: u64 = (1u64 << 48) - 1;
    const K: u64 = 0x5deece66d;
    const B: u64 = 0xb;

    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let s00 = seed;
    let s01 = 341873128712u64.wrapping_add(seed);
    let s10 = 132897987541u64.wrapping_add(seed);
    let s11 = 341873128712u64
        .wrapping_add(132897987541)
        .wrapping_add(seed);

    // p1 = nextInt(range); p2 = nextInt(range); pos = (p1+p2)>>1
    let r = sconf.region_size;
    let c = sconf.chunk_range;
    let rm = 2 * r + ((if ax < az { ax } else { az }) - 2 * radius + 7) / 8;

    let mut s = s00 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    let mut p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p <= rm {
        return 0.0;
    }
    let x0 = p;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p <= rm {
        return 0.0;
    }
    let z0 = p;

    s = s11 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p > x0 - rm {
        return 0.0;
    }
    let x1 = p;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p > z0 - rm {
        return 0.0;
    }
    let z1 = p;

    let sq = (((x1 - x0) >> 1) * ((x1 - x0) >> 1) + ((z1 - z0) >> 1) * ((z1 - z0) >> 1)) as u64;
    // C: s > (uint64_t)4*radius*radius（4 先转 u64，整体 64 位乘法）
    if sq > 4u64.wrapping_mul(radius as u64).wrapping_mul(radius as u64) {
        return 0.0;
    }

    s = s01 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p > x0 - rm {
        return 0.0;
    }
    let x2 = p;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p <= rm {
        return 0.0;
    }
    let z2 = p;

    s = s10 ^ K;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p <= rm {
        return 0.0;
    }
    let x3 = p;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p = ((s >> 17) as i32) % c;
    s = s.wrapping_mul(K).wrapping_add(B) & M;
    p += ((s >> 17) as i32) % c;
    if p > z0 - rm {
        return 0.0;
    }
    let z3 = p;

    let sqrad = get_enclosing_radius(
        x0 >> 1,
        z0 >> 1,
        x1 >> 1,
        z1 >> 1,
        x2 >> 1,
        z2 >> 1,
        x3 >> 1,
        z3 >> 1,
        ax,
        ay,
        az,
        sconf.region_size,
        radius,
    );
    if sqrad < radius as f32 { sqrad } else { 0.0 }
}

// ============================================================================
// AFK 站位优化
// ============================================================================

/// C 静态函数 `blocksInRange`：以 `(x,z)` 为球心、`rsq` 为半径平方时，
/// `n` 个结构（各 ax×az 的水平截面）落入球内的方块数。
fn blocks_in_range(p: &[Pos], x: i32, z: i32, ax: i32, az: i32, rsq: f64) -> i32 {
    let mut cnt = 0;
    for pi in p {
        let dx = (pi.x - x) as f64;
        let dz = (pi.z - z) as f64;
        for px in 0..ax {
            for pz in 0..az {
                let ddx = px as f64 + dx;
                let ddz = pz as f64 + dz;
                cnt += (ddx * ddx + ddz * ddz <= rsq) as i32;
            }
        }
    }
    cnt
}

/// C `afk_meta_t` + `checkAfkDist` 的洪水填充状态。
struct AfkMeta<'a> {
    p: &'a [Pos],
    buf: Vec<i32>,
    x0: i32,
    z0: i32,
    w: i32,
    h: i32,
    ax: i32,
    az: i32,
    rsq: f64,
    best: i32,
    sumn: i32,
    sumx: i64,
    sumz: i64,
}

impl AfkMeta<'_> {
    /// C `checkAfkDist`：洪水填充搜索更优站位（递归方向顺序与 C 一致，
    /// 访问顺序会影响 `best` 更新时机，不能随意改动）。
    fn check_afk_dist(&mut self, x: i32, z: i32) {
        if x < 0 || z < 0 || x >= self.w || z >= self.h {
            return;
        }
        if self.buf[(z * self.w + x) as usize] != 0 {
            return;
        }

        let q = blocks_in_range(self.p, x + self.x0, z + self.z0, self.ax, self.az, self.rsq);
        self.buf[(z * self.w + x) as usize] = q;
        if q >= self.best {
            if q > self.best {
                self.best = q;
                self.sumn = 1;
                self.sumx = (self.x0 + x) as i64;
                self.sumz = (self.z0 + z) as i64;
            } else {
                self.sumn += 1;
                self.sumx += (self.x0 + x) as i64;
                self.sumz += (self.z0 + z) as i64;
            }
            self.check_afk_dist(x, z - 1);
            self.check_afk_dist(x, z + 1);
            self.check_afk_dist(x - 1, z);
            self.check_afk_dist(x + 1, z);
            self.check_afk_dist(x - 1, z - 1);
            self.check_afk_dist(x - 1, z + 1);
            self.check_afk_dist(x + 1, z - 1);
            self.check_afk_dist(x + 1, z + 1);
        }
    }
}

/// `getOptimalAfk`：求四个结构（尺寸 `ax,ay,az`，位于 `p`）的最佳 AFK 位置。
///
/// 在所有「结构高度 `ay` 完全落入半径 128 球内、且水平刷怪面积最大」的整
/// 方块坐标中取平均。返回 `(站位, 可达的平面刷怪面积)`（对应 C 的返回值与
/// `spcnt` 输出参数；C 中 `spcnt` 可空，这里总是返回）。
pub fn get_optimal_afk(p: [Pos; 4], ax: i32, ay: i32, az: i32) -> (Pos, i32) {
    let mut min_x = i64::from(i32::MAX);
    let mut min_z = i64::from(i32::MAX);
    let mut max_x = i64::from(i32::MIN);
    let mut max_z = i64::from(i32::MIN);

    for pi in p {
        min_x = min_x.min(i64::from(pi.x));
        min_z = min_z.min(i64::from(pi.z));
        max_x = max_x.max(i64::from(pi.x));
        max_z = max_z.max(i64::from(pi.z));
    }

    min_x += i64::from(ax / 2);
    min_z += i64::from(az / 2);
    max_x += i64::from(ax / 2);
    max_z += i64::from(az / 2);

    let rsq = 128.0 * 128.0 - (ay * ay) as f64 / 4.0;

    let w = (max_x - min_x) as i32;
    let h = (max_z - min_z) as i32;
    let mut afk = Pos {
        x: p[0].x + ax / 2,
        z: p[0].z + az / 2,
    };
    let mut cnt = ax * az;

    let mut d = AfkMeta {
        p: &p,
        buf: vec![0; (w * h) as usize],
        x0: min_x as i32,
        z0: min_z as i32,
        w,
        h,
        ax,
        az,
        rsq,
        best: 0,
        sumn: 0,
        sumx: 0,
        sumz: 0,
    };

    let dsp = [
        Pos {
            x: (p[0].x + p[2].x) / 2,
            z: (p[0].z + p[2].z) / 2,
        },
        Pos {
            x: (p[1].x + p[3].x) / 2,
            z: (p[1].z + p[3].z) / 2,
        },
        Pos {
            x: (p[0].x + p[1].x) / 2,
            z: (p[0].z + p[1].z) / 2,
        },
        Pos {
            x: (p[2].x + p[3].x) / 2,
            z: (p[2].z + p[3].z) / 2,
        },
        Pos {
            x: (p[0].x + p[3].x) / 2,
            z: (p[0].z + p[3].z) / 2,
        },
        Pos {
            x: (p[1].x + p[2].x) / 2,
            z: (p[1].z + p[2].z) / 2,
        },
    ];
    let mut v = [0; 6];
    for i in 0..6 {
        v[i] = blocks_in_range(&p, dsp[i].x, dsp[i].z, ax, az, rsq);
    }

    for _ in 0..6 {
        // 选出最大值（严格大于，平局取先出现者）
        let mut jmax = 0;
        let mut vmax = 0;
        for (j, &vj) in v.iter().enumerate() {
            if vj > vmax {
                jmax = j;
                vmax = vj;
            }
        }
        if vmax <= ax * az {
            // 最高值不超过单个结构
            break;
        }

        d.best = vmax;
        d.sumn = 0;
        d.sumx = 0;
        d.sumz = 0;
        d.check_afk_dist(dsp[jmax].x - d.x0, dsp[jmax].z - d.z0);
        if d.best > cnt {
            cnt = d.best;
            afk.x = (d.sumx as f64 / f64::from(d.sumn)).round() as i32;
            afk.z = (d.sumz as f64 / f64::from(d.sumn)).round() as i32;
            if cnt >= 3 * ax * az {
                break;
            }
        }
        v[jmax] = 0;
    }

    (afk, cnt)
}

// ============================================================================
// 全 48 位种子搜索
// ============================================================================

/// C `searchAll48Thread` 的单线程枚举体（去掉了文件输出与断点续传）。
fn search_range<F: Fn(u64) -> bool>(
    start: u64,
    end: u64,
    low_bits: Option<(&[u64], u32)>,
    check: &F,
    stop: Option<&AtomicBool>,
    out: &mut Vec<u64>,
) {
    if let Some((bits, nbits)) = low_bits {
        let hstep = 1u64 << nbits;
        let hmask = !(hstep - 1);
        let cnt = bits.len();

        let mut mid = start & hmask;
        let mut idx = 0usize;
        let mut seed = mid | bits[idx];
        while seed < start {
            idx += 1;
            seed = mid | bits[idx];
        }

        while seed <= end {
            if check(seed) {
                out.push(seed);
            }

            idx += 1;
            if idx >= cnt {
                idx = 0;
                mid = mid.wrapping_add(hstep);
                if stop.is_some_and(|s| s.load(Ordering::Relaxed)) {
                    break;
                }
            }

            seed = mid | bits[idx];
        }
    } else {
        let mut seed = start;
        while seed <= end {
            if check(seed) {
                out.push(seed);
            }
            seed += 1;
            if seed & 0xfff == 0 && stop.is_some_and(|s| s.load(Ordering::Relaxed)) {
                break;
            }
        }
    }
}

/// `searchAll48`：多线程搜索全部 48 位种子。
///
/// 每个种子用 `check` 测试，返回 true 的记入结果。`low_bits` 为
/// `Some((值集, 位数))` 时只搜索低 `位数` 位属于值集的子集（值集与 C 不同，
/// **不含**结尾的 0）。`stop` 置位后各线程在下一轮高位步进时退出（此时返
/// 回部分结果；C 中视为错误返回 1）。
///
/// 结果按线程分区顺序拼接（与 C 的缓冲区合并行为一致）。注意：低比特子集
/// 模式下的枚举顺序是「高位块步进 × 值集数组序」，**不是**全局升序（C 同样
/// 如此）。
///
/// 与 C 的差异：不支持输出文件与断点续传；耗时长的完整搜索请注意这一点。
pub fn search_all48<F>(
    threads: usize,
    low_bits: Option<(&[u64], u32)>,
    check: F,
    stop: Option<&AtomicBool>,
) -> Vec<u64>
where
    F: Fn(u64) -> bool + Sync,
{
    assert!(threads >= 1, "search_all48: threads must be >= 1");
    const N48: u64 = 1u64 << 48;

    let mut results: Vec<Vec<u64>> = (0..threads).map(|_| Vec::new()).collect();
    let check = &check;
    let stop = &stop;
    std::thread::scope(|s| {
        for (t, out) in results.iter_mut().enumerate() {
            s.spawn(move || {
                let start = t as u64 * N48 / threads as u64;
                let end = (t as u64 + 1) * N48 / threads as u64 - 1;
                search_range(start, end, low_bits, check, *stop, out);
            });
        }
    });

    results.concat()
}

// ============================================================================
// region 区域扫描
// ============================================================================

/// C 静态函数 `scanForQuadBits`：对单个低比特取值扫描 region 矩形。
#[allow(clippy::too_many_arguments)]
fn scan_for_quad_bits(
    sconf: &StructureConfig,
    radius: i32,
    s48: u64,
    lbit: u64,
    lbitn: u32,
    inv_b: u64,
    x: i64,
    z: i64,
    w: i64,
    h: i64,
    qplist: &mut [Pos],
    n: usize,
) -> usize {
    const A: u64 = 341873128712;
    let m = 1u64 << lbitn;
    // lbitn=20 时 invB = 132477

    if n < 1 {
        return 0;
    }
    let lbit = lbit & (m - 1);

    let mut cnt = 0;
    for i in x..=(x + w) {
        let sx = s48.wrapping_add(A.wrapping_mul(i as u64));
        let j0 = ((z as u64) & !(m - 1)) | (lbit.wrapping_sub(sx).wrapping_mul(inv_b) & (m - 1));
        let mut j = j0 as i64;
        if j < z {
            j = j.wrapping_add(m as i64);
        }
        while j <= z + h {
            let sp = move_structure(s48, (-i) as i32, (-j) as i32);
            if (sp & (m - 1)) != lbit {
                j = j.wrapping_add(m as i64);
                continue;
            }

            if is_quad_base(sconf, sp, radius) != 0.0 {
                qplist[cnt] = Pos {
                    x: i as i32,
                    z: j as i32,
                };
                cnt += 1;
                if cnt >= n {
                    return cnt;
                }
            }
            j = j.wrapping_add(m as i64);
        }
    }

    cnt
}

/// `scanForQuads`：在 region 坐标矩形内扫描种子 `s48` 的四连结构。
///
/// 只对变换后低比特属于 `low_bits` 的底座逐一检查（每个星座单独考虑）。
///
/// - `radius`：传给 [`is_quad_base`] 的半径（四连小屋用 128）；
/// - `low_bits`：考虑的低比特取值集（切片，**不含** C 的结尾 0）；
/// - `low_bit_n`：子集位数（0 < low_bit_n <= 48）；
/// - `salt`：从子集取值中减去的盐（用于 protobase；一般传配置 salt）；
/// - `x,z,w,h`：扫描矩形（region 坐标，含边界，w/h 为宽高的偏移量）；
/// - `qplist`：输出 region 坐标，最多写满为止。
///
/// 返回找到的四连结构数量（不超过 `qplist.len()`）。
#[allow(clippy::too_many_arguments)]
pub fn scan_for_quads(
    sconf: &StructureConfig,
    radius: i32,
    s48: u64,
    low_bits: &[u64],
    low_bit_n: u32,
    salt: u64,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    qplist: &mut [Pos],
) -> usize {
    let inv_b = if low_bit_n == 20 {
        132477u64
    } else if low_bit_n == 48 {
        211541297333629u64
    } else {
        mul_inv(132897987541, 1u64 << low_bit_n)
    };

    let mut cnt = 0;
    for &lb in low_bits {
        let n = qplist.len() - cnt;
        cnt += scan_for_quad_bits(
            sconf,
            radius,
            s48,
            lb.wrapping_sub(salt),
            low_bit_n,
            inv_b,
            i64::from(x),
            i64::from(z),
            i64::from(w),
            i64::from(h),
            &mut qplist[cnt..],
            n,
        );
        if cnt >= qplist.len() {
            break;
        }
    }

    cnt
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `search_range` 无低比特子集：与小范围暴力枚举一致。
    #[test]
    fn search_range_plain_matches_brute_force() {
        let check = |s: u64| s % 7 == 3;
        let mut got = Vec::new();
        search_range(100, 5000, None, &check, None, &mut got);
        let want: Vec<u64> = (100..=5000).filter(|&s| check(s)).collect();
        assert_eq!(got, want);
    }

    /// `search_range` 低比特子集：枚举顺序为「高位块步进 × 值集数组序」，
    /// 起始种子按 C 的规则跳到首个 >= start 的候选。
    #[test]
    fn search_range_low_bits_order() {
        let bits: &[u64] = &[10, 3, 5]; // 故意不按序
        let (start, end) = (38u64, 300u64);
        let mut got = Vec::new();
        search_range(start, end, Some((bits, 4)), &|_| true, None, &mut got);

        // 独立复刻 C searchAll48Thread 的枚举顺序：跳过循环只跳过值集前缀，
        // 首个高位块内 idx 之后的值即使 < start 也仍会被访问（C 原样保留的
        // 怪癖，跨线程既不重复也不遗漏）。
        let mut want = Vec::new();
        let hstep = 1u64 << 4;
        let mut mid = start & !(hstep - 1);
        let mut idx = 0;
        while (mid | bits[idx]) < start {
            idx += 1;
        }
        loop {
            let s = mid | bits[idx];
            if s > end {
                break;
            }
            want.push(s);
            idx += 1;
            if idx >= bits.len() {
                idx = 0;
                mid += hstep;
            }
        }
        assert_eq!(got, want);
        // 起始低半字节 6 落在 5 与 10 之间：首个候选是 mid|10 = 42，
        // 同块内其后的 35/37（< start）按 C 怪癖仍被包含
        assert_eq!(&got[..3], &[42, 35, 37]);
    }
}
