//! Bedrock 结构散布（region 网格 + MT19937 偏移）。
//!
//! 算法与 Java 版（[`crate::structure`]）的根本差异：
//! - 随机源是 MT19937（只用 region 种子的低 32 位），不是 Java LCG；
//! - region 种子 = 世界种子(完整 64 位) + salt + rx·341873128712 + rz·132897987541
//!   （全程 wrapping i64）；
//! - 偏移为 `(mt[0..n] % separation)` 的直接或两两平均形式，且坐标是
//!   `((spacing·r + off) << 4) | 8`（i32 wrapping）。
//!
//! 以上逐行对齐 `bedrock.wasm` 的 func14（`be_find_structures`）/
//! func28（`be_get_structures_in_regions`）/ func41（`be_get_structure_config`）。

use super::mt::mt_outputs;
use super::version::BedrockVersion;

/// Bedrock 结构类型（stype 为 wasm 内部分派整数；网站未定义 stype=2）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(i32)]
pub enum BeStructureType {
    Village = 0,
    Stronghold = 1,
    DesertTemple = 3,
    WitchHut = 4,
    JungleTemple = 5,
    Igloo = 6,
    OceanMonument = 7,
    OceanRuin = 8,
    Mansion = 9,
    Shipwreck = 10,
    RuinedPortal = 11,
    BuriedTreasure = 12,
    PillagerOutpost = 13,
    NetherFortress = 14,
    Bastion = 15,
    EndCity = 16,
    AncientCity = 17,
    TrailRuin = 18,
    TrialChamber = 19,
    AbandonedCamp = 20,
}

impl BeStructureType {
    /// wasm 内部分派整数。
    #[inline]
    pub fn stype(self) -> i32 {
        self as i32
    }

    /// 全部网站可选类型（按 stype 升序，无 stype=2）。
    pub const ALL: &'static [BeStructureType] = &[
        BeStructureType::Village,
        BeStructureType::Stronghold,
        BeStructureType::DesertTemple,
        BeStructureType::WitchHut,
        BeStructureType::JungleTemple,
        BeStructureType::Igloo,
        BeStructureType::OceanMonument,
        BeStructureType::OceanRuin,
        BeStructureType::Mansion,
        BeStructureType::Shipwreck,
        BeStructureType::RuinedPortal,
        BeStructureType::BuriedTreasure,
        BeStructureType::PillagerOutpost,
        BeStructureType::NetherFortress,
        BeStructureType::Bastion,
        BeStructureType::EndCity,
        BeStructureType::AncientCity,
        BeStructureType::TrailRuin,
        BeStructureType::TrialChamber,
        BeStructureType::AbandonedCamp,
    ];
}

/// 结构散布配置（对应 wasm 数据段中每条 16 字节的配置记录）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BeStructureConfig {
    /// region 边长（单位：chunk）。
    pub spacing: i32,
    /// region 内结构区的边长（单位：chunk）。
    pub separation: i32,
    /// region 种子盐（i32，参与 i64 符号扩展加法）。
    pub salt: i32,
    /// 每个 region 消耗的 MT19937 输出个数（2 或 4；退化配置为 1）。
    pub mt_count: i32,
}

/// 退化配置：spacing=separation=1 时位置恒为每 chunk 中心，与种子无关。
pub(crate) const DEGENERATE_CONFIG: BeStructureConfig = BeStructureConfig {
    spacing: 1,
    separation: 1,
    salt: 0,
    mt_count: 1,
};

/// `be_get_structure_config`：按（版本, 结构类型）查配置表。
///
/// 逐分支对齐 wasm func41/func14 的 br_table（两者一致）：
/// 越界 stype（含 17..=20 与任何未列出的值）落到退化配置。
pub fn get_config(version: BedrockVersion, stype: BeStructureType) -> BeStructureConfig {
    get_config_raw(version.mc(), stype.stype())
}

/// 与 [`get_config`] 相同，但直接接受 wasm 整数（供测试对照 golden 配置表）。
pub fn get_config_raw(mc: i32, stype: i32) -> BeStructureConfig {
    let new = mc > 17;
    match stype {
        0 => {
            // village
            if new {
                BeStructureConfig { spacing: 34, separation: 26, salt: 10387312, mt_count: 4 }
            } else {
                BeStructureConfig { spacing: 27, separation: 17, salt: 10387312, mt_count: 4 }
            }
        }
        // stype 2 网站未定义，wasm 中与 3..=6 同配置
        2..=6 => BeStructureConfig { spacing: 32, separation: 24, salt: 14357617, mt_count: 2 },
        7 => BeStructureConfig { spacing: 32, separation: 27, salt: 10387313, mt_count: 4 },
        8 => {
            // ocean_ruin
            if new {
                BeStructureConfig { spacing: 20, separation: 12, salt: 14357621, mt_count: 2 }
            } else {
                BeStructureConfig { spacing: 12, separation: 5, salt: 14357621, mt_count: 4 }
            }
        }
        9 => BeStructureConfig { spacing: 80, separation: 60, salt: 10387319, mt_count: 4 },
        10 => {
            // shipwreck
            if new {
                BeStructureConfig { spacing: 24, separation: 20, salt: 165745295, mt_count: 2 }
            } else {
                BeStructureConfig { spacing: 10, separation: 5, salt: 165745295, mt_count: 4 }
            }
        }
        11 => BeStructureConfig { spacing: 40, separation: 25, salt: 40552231, mt_count: 2 },
        12 => BeStructureConfig { spacing: 4, separation: 2, salt: 16842397, mt_count: 4 },
        13 => BeStructureConfig { spacing: 80, separation: 56, salt: 165745296, mt_count: 4 },
        14 | 15 => BeStructureConfig { spacing: 30, separation: 26, salt: 30084232, mt_count: 2 },
        16 => BeStructureConfig { spacing: 20, separation: 9, salt: 10387313, mt_count: 4 },
        _ => DEGENERATE_CONFIG,
    }
}

/// 单个 region 内的结构候选位置（`bedrock.wasm` func14 内层循环体）。
///
/// `rx`,`rz` 为 region 坐标，返回该 region 的结构方块坐标。
/// 退化配置（separation=1）下与种子无关，恒为 region（=chunk）中心。
pub fn get_structure_pos(config: &BeStructureConfig, seed: i64, rx: i32, rz: i32) -> [i32; 2] {
    // region 种子：i64 wrapping 运算（wasm 的 i64.add/mul 语义）
    let region_seed = seed
        .wrapping_add(config.salt as i64)
        .wrapping_add((rx as i64).wrapping_mul(341873128712))
        .wrapping_add((rz as i64).wrapping_mul(132897987541));

    let (xoff, zoff) = if config.separation == 1 {
        // 退化配置：wasm 中此处会越界读 MT 输出，但 % 1 恒为 0
        (0, 0)
    } else {
        let sep = config.separation as u32;
        let mt = mt_outputs(region_seed as u32, config.mt_count as usize);
        if config.mt_count == 2 {
            ((mt[0] % sep) as i32, (mt[1] % sep) as i32)
        } else {
            (
                ((mt[1] % sep).wrapping_add(mt[0] % sep) >> 1) as i32,
                ((mt[3] % sep).wrapping_add(mt[2] % sep) >> 1) as i32,
            )
        }
    };

    let x = (config.spacing.wrapping_mul(rx).wrapping_add(xoff) << 4) | 8;
    let z = (config.spacing.wrapping_mul(rz).wrapping_add(zoff) << 4) | 8;
    [x, z]
}

/// 由 range 参数计算 region 半径（wasm func14 开头，C 截断除法语义）。
///
/// `range_scaled` 为 wasm 第 7 参数（l 包装器传入 `range << 9`）。
fn region_radius(range_scaled: i32, spacing: i32) -> i32 {
    let raw = range_scaled / (spacing << 4) + i32::from(range_scaled % spacing != 0);
    let t = raw - i32::from(raw > 0);
    let t = if t > 0 { t } else { 0 };
    t.min(100)
}

/// func14 的中心方块坐标 → 中心 region 坐标换算（截断除法）。
fn center_region(c: i32, spacing: i32) -> i32 {
    let e = if c < 0 { c - 15 } else { c };
    let j = e / 16;
    (if e < -15 { j - spacing + 1 } else { j }) / spacing
}

/// func14 主体：以 region `(brx, brz)` 为中心、半径 `rad` 的网格内全部结构位置。
#[allow(clippy::too_many_arguments)]
fn scatter(
    config: &BeStructureConfig,
    seed: i64,
    brx: i32,
    brz: i32,
    rad: i32,
) -> Vec<[i32; 2]> {
    if config.spacing <= 0 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(((2 * rad + 1) * (2 * rad + 1)) as usize);
    for i in -rad..=rad {
        for j in -rad..=rad {
            let rx = brx.wrapping_add(i);
            let rz = brz.wrapping_add(j);
            out.push(get_structure_pos(config, seed, rx, rz));
        }
    }
    out
}

/// `be_get_structures_in_regions`（wasm func28 = func14 的 l 包装）：
/// 以原点为中心、±`range` region 的网格内全部结构位置。
///
/// `range` 语义与网站一致（wasm 内部左移 9 位成方块半径）。
pub fn structures_in_regions(
    version: BedrockVersion,
    stype: BeStructureType,
    seed: i64,
    range: i32,
) -> Vec<[i32; 2]> {
    find_structures(version, stype, seed, 0, 0, range)
}

/// `be_find_structures`（wasm func14）：以方块坐标 `(cx, cz)` 为中心、
/// `range << 9` 方块半径内的全部结构位置。
pub fn find_structures(
    version: BedrockVersion,
    stype: BeStructureType,
    seed: i64,
    cx: i32,
    cz: i32,
    range: i32,
) -> Vec<[i32; 2]> {
    let config = get_config(version, stype);
    if config.spacing <= 0 {
        return Vec::new();
    }
    let rad = region_radius(range << 9, config.spacing);
    let brx = center_region(cx, config.spacing);
    let brz = center_region(cz, config.spacing);
    scatter(&config, seed, brx, brz, rad)
}

// ---- 群系过滤版（`be_get_filtered_structures_in_regions`，wasm func21）----
//
// func21 先调 func14 得到全部候选，再按结构类型做群系可行性过滤
// （br_table 分派；无规则的类型直通）。过滤用层栈即 [`LayerStack`]
// （54 层 Bedrock 群系层链，与版本无关，全版本共用同一层栈）。
//
// 注意：mcseedmap.com 自身未启用此版（其 bedrock-worker.js 注释说明
// 地图底图复用 Java 引擎，群系上下文由底图提供），此处为完整性移植。

use super::layers::LayerStack;

/// 单条过滤规则：以候选方块坐标为中心、`radius` 方块半径内的所有 scale-4
/// 格子群系都必须属于 `biomes`（对应 wasm f_f 调用）。
struct FilterRule {
    radius: i32,
    biomes: &'static [i32],
}

/// 各结构类型的过滤规则表（func21 br_table + wasm 数据段的 -1 终止列表）。
/// 海底神殿为两段与：r=16 深海集合 && r=29 全部海洋+河流。
fn filter_rules(stype: BeStructureType) -> &'static [FilterRule] {
    match stype {
        BeStructureType::Village => &[FilterRule {
            radius: 2,
            biomes: &[1, 35, 12, 5, 19, 30, 31, 2],
        }],
        BeStructureType::DesertTemple => &[FilterRule {
            radius: 0,
            biomes: &[2, 17, 130],
        }],
        BeStructureType::WitchHut => &[FilterRule {
            radius: 0,
            // wasm 列表为 [6, 6]（重复项，原样保留语义）
            biomes: &[6, 6],
        }],
        BeStructureType::JungleTemple => &[FilterRule {
            radius: 0,
            biomes: &[21, 22],
        }],
        BeStructureType::Igloo => &[FilterRule {
            radius: 0,
            biomes: &[12, 30],
        }],
        BeStructureType::OceanMonument => &[
            FilterRule {
                radius: 16,
                biomes: &[24, 46, 42, 48, 44],
            },
            FilterRule {
                radius: 29,
                biomes: &[0, 24, 43, 45, 47, 41, 44, 42, 48, 46, 7, 11],
            },
        ],
        BeStructureType::Mansion => &[FilterRule {
            radius: 32,
            biomes: &[29],
        }],
        BeStructureType::BuriedTreasure => &[FilterRule {
            radius: 3,
            biomes: &[16, 26, 25, 15],
        }],
        BeStructureType::PillagerOutpost => &[FilterRule {
            radius: 0,
            biomes: &[1, 129, 35, 12, 19, 5, 30, 31, 2],
        }],
        _ => &[],
    }
}

/// `be_get_filtered_structures_in_regions`（wasm func21）：
/// 与 [`structures_in_regions`] 相同，但只保留通过群系可行性过滤的候选。
///
/// 无过滤规则的类型（要塞、海底遗迹、沉船、废弃传送门、下界/末地结构等）
/// 结果与非过滤版完全一致。mcseedmap.com 自身未启用此版。
pub fn structures_in_regions_filtered(
    version: BedrockVersion,
    stype: BeStructureType,
    seed: i64,
    range: i32,
) -> Vec<[i32; 2]> {
    let candidates = structures_in_regions(version, stype, seed, range);
    let rules = filter_rules(stype);
    if rules.is_empty() {
        return candidates;
    }
    let stack = LayerStack::new(seed);
    candidates
        .into_iter()
        .filter(|[x, z]| rules.iter().all(|r| stack.check(*x, *z, r.radius, r.biomes)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degenerate_grid_is_seed_independent() {
        // 退化类型：每个 chunk 中心一处，与种子无关
        for &st in &[
            BeStructureType::Stronghold,
            BeStructureType::AncientCity,
            BeStructureType::TrailRuin,
            BeStructureType::TrialChamber,
            BeStructureType::AbandonedCamp,
        ] {
            let a = structures_in_regions(BedrockVersion::V1_21_0, st, 0, 1);
            let b = structures_in_regions(BedrockVersion::V1_21_0, st, -99999, 1);
            assert_eq!(a, b);
            // range=1 → 半径 (512/(1*16)) + 0 = 32 → rad=31 → 63x63
            assert_eq!(a.len(), 63 * 63);
            assert!(a.contains(&[8, 8]));
            assert!(a.contains(&[-8, -8]));
        }
    }

    #[test]
    fn region_radius_matches_wasm() {
        // village mc<=17 spacing=27, range=2 → 1024/432=2, 1024%27!=0 → 3 → rad=2
        assert_eq!(region_radius(2 << 9, 27), 2);
        // 退化 spacing=1：raw=512/16=32, rad=31；range=4 时 raw=128 → rad=min(127,100)=100
        assert_eq!(region_radius(1 << 9, 1), 31);
        assert_eq!(region_radius(4 << 9, 1), 100);
        // buried_treasure spacing=4：1024/64=16, 1024%4==0 → 16 → rad=15
        assert_eq!(region_radius(2 << 9, 4), 15);
    }
}
