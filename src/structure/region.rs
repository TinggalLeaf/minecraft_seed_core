//! region 候选定位：移植 cubiomes `finders.c`/`finders.h` 的
//! `getStructurePos`、`getMineshafts`、`getPopulationSeed`、`isSlimeChunk` 等。
//!
//! ## 结构定位原理（译自 C 头文件注释）
//!
//! 大多数结构把世界划分为 region 网格（通常 32×32 区块），每个 region 内做
//! 一次生成尝试。尝试位置只取决于结构类型、region 坐标与世界种子的低 48 位，
//! 高 16 位不影响结构位置。位置对 region 坐标是线性的，可用
//! [`move_structure`] 把种子平移 `(dregX, dregZ)` 个 region：
//!
//! ```text
//! seed2 = seed1 - dregX * 341873128712 - dregZ * 132897987541
//! ```
//!
//! ## C 侧怪癖备忘
//!
//! - `getFeatureChunkInRegion`/`getLargeStructureChunkInRegion` 是 cubiomes 的
//!   内联快路径：非 2 次幂的 `chunkRange` 直接取 `next(31) % r`，**省略了**
//!   `java.util.Random.nextInt` 的拒绝采样（概率约 2³¹ mod r / 2³¹，亿万分之
//!   几）。为与 cubiomes（及 mcseedmap）逐位一致，此处原样保留该行为。
//! - C 中大量 `int * uint64_t` 混合运算依赖隐式符号扩展与 2 的幂回绕，
//!   这里全部写成显式 `wrapping_*` + `as` 转换。

use crate::rng::{JavaRandom, Xoroshiro};
use crate::version::McVersion;

use super::config::{get_config, StructureConfig, StructureType};

/// 2D 方块坐标（对应 C `Pos`）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pos {
    pub x: i32,
    pub z: i32,
}

/// 48 位掩码（C `MASK48`）。
const MASK48: u64 = (1u64 << 48) - 1;

/// `moveStructure`：把结构基准种子平移 `(reg_x, reg_z)` 个 region。
#[inline]
pub fn move_structure(base_seed: u64, reg_x: i32, reg_z: i32) -> u64 {
    base_seed
        .wrapping_sub((reg_x as i64 as u64).wrapping_mul(341873128712))
        .wrapping_sub((reg_z as i64 as u64).wrapping_mul(132897987541))
        & MASK48
}

/// `getShadow`：影子种子（末地黑曜石柱等使用）。
#[inline]
pub fn get_shadow(seed: u64) -> u64 {
    (-7379792620528906219i64 as u64).wrapping_sub(seed)
}

/// `setAttemptSeed`：1.14+ 结构尝试的二次种子（要塞/前哨站等）。
///
/// 在 `s` 的原值基础上异或区块坐标后重设 LCG 并丢弃一次 `next(31)`。
pub(crate) fn set_attempt_seed(s: &mut u64, cx: i32, cz: i32) -> JavaRandom {
    *s ^= (cx >> 4) as i64 as u64 ^ (((cz >> 4) as i64 as u64) << 4);
    let mut r = JavaRandom::new(0);
    r.set_seed(*s as i64);
    r.next(31);
    r
}

/// `chunkGenerateRnd`：16×16 区块生成的初始随机源（洞穴/废弃矿井/结构
/// 部件等的 `recursiveGenerate` 用）。
#[inline]
pub fn chunk_generate_rnd(world_seed: u64, chunk_x: i32, chunk_z: i32) -> JavaRandom {
    let mut r = JavaRandom::new(world_seed as i64);
    let a = r.next_long();
    let b = r.next_long();
    let s = (a as u64)
        .wrapping_mul(chunk_x as i64 as u64)
        ^ (b as u64).wrapping_mul(chunk_z as i64 as u64)
        ^ world_seed;
    JavaRandom::new(s as i64)
}

/// `getPopulationSeed`：装饰性 feature（1.13+ 的地表装饰）的区块种子。
pub fn get_population_seed(mc: McVersion, ws: u64, x: i32, z: i32) -> u64 {
    let (mut a, mut b);
    if mc >= McVersion::V1_18 {
        let mut xr = Xoroshiro::new(ws);
        a = xr.next_long_j();
        b = xr.next_long_j();
    } else {
        let mut s = JavaRandom::new(ws as i64);
        a = s.next_long() as u64;
        b = s.next_long() as u64;
    }
    if mc >= McVersion::V1_13 {
        a |= 1;
        b |= 1;
    } else {
        // C: (int64_t)a / 2 * 2 + 1（截断除法 + 回绕乘加）
        a = ((a as i64) / 2).wrapping_mul(2).wrapping_add(1) as u64;
        b = ((b as i64) / 2).wrapping_mul(2).wrapping_add(1) as u64;
    }
    (x as i64 as u64)
        .wrapping_mul(a)
        .wrapping_add((z as i64 as u64).wrapping_mul(b))
        ^ ws
}

/// `getFeatureChunkInRegion`：小型结构的 region 内区块偏移（均匀分布）。
///
/// 与 C 内联版一致：对 `seed + regX*A + regZ*B + salt` 直接推进 LCG，
/// 非 2 次幂范围不做拒绝采样（见模块文档）。
pub fn get_feature_chunk_in_region(
    config: &StructureConfig,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Pos {
    const K: u64 = 0x5deece66d;
    const B: u64 = 0xb;

    // set seed（C 此处异或后不显式掩码，低 48 位之外不影响后续状态）
    let mut s = seed
        .wrapping_add((reg_x as i64 as u64).wrapping_mul(341873128712))
        .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
        .wrapping_add(config.salt as i64 as u64);
    s ^= K;
    s = s.wrapping_mul(K).wrapping_add(B) & MASK48;

    let r = config.chunk_range as u64;
    let mut pos = Pos::default();
    if r & (r - 1) != 0 {
        pos.x = ((s >> 17) % r) as i32;
        s = s.wrapping_mul(K).wrapping_add(B) & MASK48;
        pos.z = ((s >> 17) % r) as i32;
    } else {
        // Java RNG 对 2 的幂有特殊路径
        pos.x = (r.wrapping_mul(s >> 17) >> 31) as i32;
        s = s.wrapping_mul(K).wrapping_add(B) & MASK48;
        pos.z = (r.wrapping_mul(s >> 17) >> 31) as i32;
    }
    pos
}

/// `getFeaturePos`：小型结构在 region 内的方块坐标。
pub fn get_feature_pos(config: &StructureConfig, seed: u64, reg_x: i32, reg_z: i32) -> Pos {
    let pos = get_feature_chunk_in_region(config, seed, reg_x, reg_z);
    Pos {
        x: (((reg_x as i64 as u64).wrapping_mul(config.region_size as u64))
            .wrapping_add(pos.x as u64)
            << 4) as i32,
        z: (((reg_z as i64 as u64).wrapping_mul(config.region_size as u64))
            .wrapping_add(pos.z as u64)
            << 4) as i32,
    }
}

/// `getLargeStructureChunkInRegion`：大型结构（海底神殿/林地府邸/末地城）
/// 的 region 内区块偏移（三角分布）。
pub fn get_large_structure_chunk_in_region(
    config: &StructureConfig,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Pos {
    const K: u64 = 0x5deece66d;
    const B: u64 = 0xb;

    // C 备注：2 次幂 chunkRange 未特殊处理（现有配置均非 2 次幂）
    let mut s = seed
        .wrapping_add((reg_x as i64 as u64).wrapping_mul(341873128712))
        .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
        .wrapping_add(config.salt as i64 as u64);
    s ^= K;

    let r = config.chunk_range as u64;
    let mut pos = Pos::default();
    s = s.wrapping_mul(K).wrapping_add(B) & MASK48;
    pos.x = ((s >> 17) % r) as i32;
    s = s.wrapping_mul(K).wrapping_add(B) & MASK48;
    pos.x += ((s >> 17) % r) as i32;

    s = s.wrapping_mul(K).wrapping_add(B) & MASK48;
    pos.z = ((s >> 17) % r) as i32;
    s = s.wrapping_mul(K).wrapping_add(B) & MASK48;
    pos.z += ((s >> 17) % r) as i32;

    pos.x >>= 1;
    pos.z >>= 1;
    pos
}

/// `getLargeStructurePos`：大型结构在 region 内的方块坐标。
pub fn get_large_structure_pos(
    config: &StructureConfig,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Pos {
    let pos = get_large_structure_chunk_in_region(config, seed, reg_x, reg_z);
    Pos {
        x: (((reg_x as i64 as u64).wrapping_mul(config.region_size as u64))
            .wrapping_add(pos.x as u64)
            << 4) as i32,
        z: (((reg_z as i64 as u64).wrapping_mul(config.region_size as u64))
            .wrapping_add(pos.z as u64)
            << 4) as i32,
    }
}

/// `getRegPos`：类 `getFeaturePos`，但返回推进后的 LCG（供后续判定使用）。
fn get_reg_pos(world_seed: u64, rx: i32, rz: i32, sc: &StructureConfig) -> (Pos, JavaRandom) {
    let mut r = JavaRandom::new(0);
    r.set_seed(
        (rx as i64 as u64)
            .wrapping_mul(341873128712)
            .wrapping_add((rz as i64 as u64).wrapping_mul(132897987541))
            .wrapping_add(world_seed)
            .wrapping_add(sc.salt as i64 as u64) as i64,
    );
    let pos = Pos {
        x: (((rx as i64 as u64).wrapping_mul(sc.region_size as u64))
            .wrapping_add(r.next_int_bound(sc.chunk_range) as u64)
            << 4) as i32,
        z: (((rz as i64 as u64).wrapping_mul(sc.region_size as u64))
            .wrapping_add(r.next_int_bound(sc.chunk_range) as u64)
            << 4) as i32,
    };
    (pos, r)
}

/// `getMineshafts`：检查从 `(cx0, cz0)` 到 `(cx1, cz1)`（含）的区块矩形内
/// 的废弃矿井位置。
///
/// `out` 为 `Some` 时按序写入至多 `out.len()` 个位置；返回矩形内矿井总数
/// （可能超过写入数）。
pub fn get_mineshafts(
    mc: McVersion,
    seed: u64,
    cx0: i32,
    cz0: i32,
    cx1: i32,
    cz1: i32,
    mut out: Option<&mut [Pos]>,
) -> i32 {
    let mut s = JavaRandom::new(seed as i64);
    let a = s.next_long() as u64;
    let b = s.next_long() as u64;
    let mut n = 0i32;

    for i in cx0..=cx1 {
        let aix = (i as i64 as u64).wrapping_mul(a) ^ seed;
        for j in cz0..=cz1 {
            let mut s = JavaRandom::new(0);
            s.set_seed((aix ^ (j as i64 as u64).wrapping_mul(b)) as i64);

            if mc >= McVersion::V1_13 {
                if s.next_double() < 0.004 {
                    if let Some(buf) = out.as_deref_mut()
                        && (n as usize) < buf.len()
                    {
                        buf[n as usize] = Pos { x: i * 16, z: j * 16 };
                    }
                    n += 1;
                }
            } else {
                s.skip(1);
                if s.next_double() < 0.004 {
                    // C: d = max(i, -i, j, -j)（即 max(|i|, |j|)，INT_MIN 除外）
                    let mut d = i;
                    if -i > d {
                        d = -i;
                    }
                    if j > d {
                        d = j;
                    }
                    if -j > d {
                        d = -j;
                    }
                    if d >= 80 || s.next_int_bound(80) < d {
                        if let Some(buf) = out.as_deref_mut()
                            && (n as usize) < buf.len()
                        {
                            buf[n as usize] = Pos { x: i * 16, z: j * 16 };
                        }
                        n += 1;
                    }
                }
            }
        }
    }
    n
}

/// `getStructurePos`：求某 region 内结构生成尝试的方块坐标。
///
/// 有些结构在给定 region 内无论群系如何都不会生成（如未通过稀有度判定），
/// 此时返回 `None`。群系可行性由
/// [`super::viability::is_viable_structure_pos`] 进一步检查。
///
/// `seed` 只有低 48 位有效。
pub fn get_structure_pos(
    stype: StructureType,
    mc: McVersion,
    seed: u64,
    reg_x: i32,
    reg_z: i32,
) -> Option<Pos> {
    use StructureType::*;
    let sconf = get_config(stype, mc)?;

    match stype {
        Feature | DesertPyramid | JungleTemple | SwampHut | Igloo | Village | OceanRuin
        | Shipwreck | RuinedPortal | RuinedPortalN | AncientCity | TrailRuins
        | TrialChambers => Some(get_feature_pos(&sconf, seed, reg_x, reg_z)),

        Monument | Mansion => Some(get_large_structure_pos(&sconf, seed, reg_x, reg_z)),

        EndCity => {
            let pos = get_large_structure_pos(&sconf, seed, reg_x, reg_z);
            let r2 = pos.x as i64 * pos.x as i64 + pos.z as i64 * pos.z as i64;
            (r2 >= 1008 * 1008).then_some(pos)
        }

        Outpost => {
            let pos = get_feature_pos(&sconf, seed, reg_x, reg_z);
            let mut s = seed;
            let mut r = set_attempt_seed(&mut s, pos.x >> 4, pos.z >> 4);
            (r.next_int_bound(5) == 0).then_some(pos)
        }

        Treasure => {
            let pos = Pos {
                x: reg_x * 16 + 9,
                z: reg_z * 16 + 9,
            };
            let s = (reg_x as i64 as u64)
                .wrapping_mul(341873128712)
                .wrapping_add((reg_z as i64 as u64).wrapping_mul(132897987541))
                .wrapping_add(seed)
                .wrapping_add(sconf.salt as i64 as u64);
            let mut r = JavaRandom::new(s as i64);
            (r.next_float() < 0.01).then_some(pos)
        }

        Mineshaft => {
            let mut buf = [Pos::default()];
            let n = get_mineshafts(mc, seed, reg_x, reg_z, reg_x, reg_z, Some(&mut buf));
            (n > 0).then_some(buf[0])
        }

        Fortress => {
            if mc >= McVersion::V1_18 {
                // 1.18+ 要塞生成在堡垒遗迹不生成的群系（群系决定，见 viability）
                Some(get_feature_pos(&sconf, seed, reg_x, reg_z))
            } else if mc >= McVersion::V1_16_1 {
                let (pos, mut r) = get_reg_pos(seed, reg_x, reg_z, &sconf);
                (r.next_int_bound(5) < 2).then_some(pos)
            } else {
                let mut s = seed;
                let mut r = set_attempt_seed(&mut s, reg_x * 16, reg_z * 16);
                let valid = r.next_int_bound(3) == 0;
                let pos = Pos {
                    x: (reg_x * 16 + r.next_int_bound(8) + 4) * 16,
                    z: (reg_z * 16 + r.next_int_bound(8) + 4) * 16,
                };
                valid.then_some(pos)
            }
        }

        Bastion => {
            if mc >= McVersion::V1_18 {
                let pos = get_feature_pos(&sconf, seed, reg_x, reg_z);
                let mut r = chunk_generate_rnd(seed, pos.x >> 4, pos.z >> 4);
                (r.next_int_bound(5) >= 2).then_some(pos)
            } else {
                let (pos, mut r) = get_reg_pos(seed, reg_x, reg_z, &sconf);
                (r.next_int_bound(5) >= 2).then_some(pos)
            }
        }

        // 装饰性 feature（区域 = 单区块）
        EndGateway | EndIsland | DesertWell | Geode => {
            let mut pos = Pos {
                x: reg_x * 16,
                z: reg_z * 16,
            };
            let s = get_population_seed(mc, seed, pos.x, pos.z);
            if mc >= McVersion::V1_18 {
                let mut xr = Xoroshiro::new(s.wrapping_add(sconf.salt as i64 as u64));
                if xr.next_float() >= sconf.rarity {
                    return None;
                }
                pos.x += xr.next_int_j(16);
                pos.z += xr.next_int_j(16);
            } else {
                let mut r = JavaRandom::new(s.wrapping_add(sconf.salt as i64 as u64) as i64);
                if sconf.rarity < 1.0 {
                    if r.next_float() >= sconf.rarity {
                        return None;
                    }
                } else if r.next_int_bound(sconf.rarity as i32) != 0 {
                    return None;
                }
                pos.x += r.next_int_bound(16);
                pos.z += r.next_int_bound(16);
            }
            Some(pos)
        }
    }
}

/// `isSlimeChunk`：史莱姆区块判定（Java 版）。
///
/// C 中 `chunkX * 0x5ac0db` 等为 32 位 `int` 回绕乘法，这里用
/// `wrapping_mul` 复刻。
#[inline]
pub fn is_slime_chunk(seed: u64, chunk_x: i32, chunk_z: i32) -> bool {
    let mut rnd = seed;
    rnd = rnd.wrapping_add(chunk_x.wrapping_mul(0x5ac0db) as i64 as u64);
    rnd = rnd.wrapping_add(
        chunk_x
            .wrapping_mul(chunk_x)
            .wrapping_mul(0x4c1906) as i64 as u64,
    );
    rnd = rnd.wrapping_add(chunk_z.wrapping_mul(0x5f24f) as i64 as u64);
    rnd = rnd.wrapping_add(
        (chunk_z.wrapping_mul(chunk_z) as i64 as u64).wrapping_mul(0x4307a7),
    );
    rnd ^= 0x3ad8025f;
    JavaRandom::new(rnd as i64).next_int_bound(10) == 0
}

/// 末地小岛（`EndIsland`）：`getEndIslands` 的输出元素。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EndIsland {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub r: i32,
}

/// `getEndIslands`：求给定区块内末地小岛的位置与半径。
///
/// 返回 0–2 个小岛（对应 C 写入 `islands[2]` 的返回值）。
pub fn get_end_islands(mc: McVersion, seed: u64, chunk_x: i32, chunk_z: i32) -> Vec<EndIsland> {
    let sconf = match get_config(StructureType::EndIsland, mc) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let x = chunk_x * 16;
    let z = chunk_z * 16;
    let rng = get_population_seed(mc, seed, x, z);

    if mc <= McVersion::V1_16 {
        let mut r = JavaRandom::new(rng.wrapping_add(sconf.salt as i64 as u64) as i64);
        if r.next_int_bound(sconf.rarity as i32) != 0 {
            return Vec::new();
        }
        let mut is0 = EndIsland {
            x: r.next_int_bound(16) + x,
            y: r.next_int_bound(16) + 55,
            z: r.next_int_bound(16) + z,
            r: 0,
        };
        if r.next_int_bound(4) != 0 {
            is0.r = r.next_int_bound(3) + 4;
            return vec![is0];
        }
        let mut is1 = EndIsland {
            x: r.next_int_bound(16) + x,
            y: r.next_int_bound(16) + 55,
            z: r.next_int_bound(16) + z,
            r: 0,
        };
        is0.r = r.next_int_bound(3) + 4;
        // C 的收缩循环只消耗随机数（islands[0].r 保持初值不变）
        let mut fr = is0.r as f32;
        while fr > 0.5 {
            fr -= r.next_int_bound(2) as f32 + 0.5;
        }
        is1.r = r.next_int_bound(3) + 4;
        vec![is0, is1]
    } else if mc <= McVersion::V1_17 {
        let mut r = JavaRandom::new(rng.wrapping_add(sconf.salt as i64 as u64) as i64);
        if r.next_float() >= sconf.rarity {
            return Vec::new();
        }
        let second = r.next_int_bound(4) == 0;
        let is0 = EndIsland {
            x: r.next_int_bound(16) + x,
            z: r.next_int_bound(16) + z,
            y: r.next_int_bound(16) + 55,
            r: r.next_int_bound(3) + 4,
        };
        let mut fr = is0.r as f32;
        while fr > 0.5 {
            fr -= r.next_int_bound(2) as f32 + 0.5;
        }
        if !second {
            return vec![is0];
        }
        let is1 = EndIsland {
            x: r.next_int_bound(16) + x,
            z: r.next_int_bound(16) + z,
            y: r.next_int_bound(16) + 55,
            r: r.next_int_bound(3) + 4,
        };
        vec![is0, is1]
    } else {
        let mut xr = Xoroshiro::new(rng.wrapping_add(sconf.salt as i64 as u64));
        if xr.next_float() >= sconf.rarity {
            return Vec::new();
        }
        let second = xr.next_int_j(4) == 3;
        let is0 = EndIsland {
            x: xr.next_int_j(16) + x,
            z: xr.next_int_j(16) + z,
            y: xr.next_int_j(16) + 55,
            r: xr.next_int_j(3) + 4,
        };
        if !second {
            return vec![is0];
        }
        let mut fr = is0.r as f32;
        while fr > 0.5 {
            fr -= xr.next_int_j(2) as f32 + 0.5;
        }
        let is1 = EndIsland {
            x: xr.next_int_j(16) + x,
            z: xr.next_int_j(16) + z,
            y: xr.next_int_j(16) + 55,
            r: xr.next_int_j(3) + 4,
        };
        vec![is0, is1]
    }
}
