//! 结构类型与按版本的生成配置表，移植自 cubiomes `finders.c` 的
//! `getStructureConfig`。
//!
//! 配置表是结构查找正确性的关键：每种结构在每个版本区间的
//! spacing（region 边长，单位区块）/ separation（region 内随机偏移范围）
//! / salt（种子盐）/ 维度 / 稀有度 都必须与 C 逐项一致。golden 测试对
//! 全部版本 × 全部结构类型做了快照比较（见 `super::tests`）。

use crate::version::McVersion;

/// 结构类型（判别值与 cubiomes `enum StructureType` 一致）。
///
/// `JunglePyramid` 与 `JungleTemple` 在 C 中是同一枚举值的两个名字，
/// 这里统一为 `JungleTemple`。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum StructureType {
    /// 1.13 之前神殿类结构的统一生成尝试（沙漠神殿/丛林神庙/沼泽小屋/雪屋共用）。
    Feature = 0,
    DesertPyramid = 1,
    JungleTemple = 2,
    SwampHut = 3,
    Igloo = 4,
    Village = 5,
    OceanRuin = 6,
    Shipwreck = 7,
    Monument = 8,
    Mansion = 9,
    Outpost = 10,
    RuinedPortal = 11,
    /// 下界废弃传送门（1.17- 的 spacing 与主世界不同；C 的
    /// `s_ruined_portal_n` 把 structType 记为 `Ruined_Portal`）。
    RuinedPortalN = 12,
    AncientCity = 13,
    Treasure = 14,
    Mineshaft = 15,
    DesertWell = 16,
    Geode = 17,
    Fortress = 18,
    Bastion = 19,
    EndCity = 20,
    EndGateway = 21,
    EndIsland = 22,
    TrailRuins = 23,
    TrialChambers = 24,
}

/// `FEATURE_NUM`：结构类型总数。
pub const FEATURE_NUM: usize = 25;

impl StructureType {
    /// 全部结构类型（按 C 枚举序）。
    pub const ALL: &'static [StructureType] = &[
        StructureType::Feature,
        StructureType::DesertPyramid,
        StructureType::JungleTemple,
        StructureType::SwampHut,
        StructureType::Igloo,
        StructureType::Village,
        StructureType::OceanRuin,
        StructureType::Shipwreck,
        StructureType::Monument,
        StructureType::Mansion,
        StructureType::Outpost,
        StructureType::RuinedPortal,
        StructureType::RuinedPortalN,
        StructureType::AncientCity,
        StructureType::Treasure,
        StructureType::Mineshaft,
        StructureType::DesertWell,
        StructureType::Geode,
        StructureType::Fortress,
        StructureType::Bastion,
        StructureType::EndCity,
        StructureType::EndGateway,
        StructureType::EndIsland,
        StructureType::TrailRuins,
        StructureType::TrialChambers,
    ];

    /// 从 C 枚举值转换（供 golden 测试对照）。
    pub fn from_u8(v: u8) -> Option<StructureType> {
        if (v as usize) < FEATURE_NUM {
            // 判别值连续且与索引一致
            Some(Self::ALL[v as usize])
        } else {
            None
        }
    }
}

/// 结构生成配置（对应 C `StructureConfig`）。
///
/// - `salt`：混入 region 种子的结构盐。
/// - `region_size`：region 边长（单位：区块）；C 的 `regionSize`。
/// - `chunk_range`：region 内随机偏移范围（单位：区块）；C 的 `chunkRange`。
/// - `struct_type`：C 原样保留的 structType 字段（注意 `s_ruined_portal_n`
///   记的是 `RuinedPortal` 而非 `RuinedPortalN`）。
/// - `dim`：维度（cubiomes `Dimension` 数值：-1 下界 / 0 主世界 / +1 末地）。
/// - `rarity`：稀有度。`< 1.0` 为概率（`nextFloat < rarity`），`>= 1.0`
///   为 1/N 判定（`nextInt(N) == 0`），`0.0` 表示总是生成。
#[derive(Clone, Copy, Debug)]
pub struct StructureConfig {
    pub salt: i32,
    pub region_size: i32,
    pub chunk_range: i32,
    pub struct_type: StructureType,
    pub dim: i8,
    pub rarity: f32,
}

impl StructureConfig {
    /// C 结构体字面量的便捷构造（`{salt, regionSize, chunkRange, type, dim, rarity}`）。
    const fn c(
        salt: i32,
        region_size: i8,
        chunk_range: i8,
        struct_type: StructureType,
        dim: i8,
        rarity: f32,
    ) -> Self {
        StructureConfig {
            salt,
            region_size: region_size as i32,
            chunk_range: chunk_range as i32,
            struct_type,
            dim,
            rarity,
        }
    }
}

/// 下界维度（C `DIM_NETHER`）。
const DIM_NETHER: i8 = -1;
/// 末地维度（C `DIM_END`）。
const DIM_END: i8 = 1;

// ============================================================================
// 配置常量表（与 `getStructureConfig` 的 static const 表逐项对应）
// ============================================================================

// 1.13 之前沙漠神殿/丛林神庙/沼泽小屋/雪屋共用的 feature 配置
const S_FEATURE: StructureConfig =
    StructureConfig::c(14357617, 32, 24, StructureType::Feature, 0, 0.0);
const S_IGLOO_112: StructureConfig =
    StructureConfig::c(14357617, 32, 24, StructureType::Igloo, 0, 0.0);
const S_SWAMP_HUT_112: StructureConfig =
    StructureConfig::c(14357617, 32, 24, StructureType::SwampHut, 0, 0.0);
const S_DESERT_PYRAMID_112: StructureConfig =
    StructureConfig::c(14357617, 32, 24, StructureType::DesertPyramid, 0, 0.0);
const S_JUNGLE_TEMPLE_112: StructureConfig =
    StructureConfig::c(14357617, 32, 24, StructureType::JungleTemple, 0, 0.0);
// 1.16 之前的海洋结构
const S_OCEAN_RUIN_115: StructureConfig =
    StructureConfig::c(14357621, 16, 8, StructureType::OceanRuin, 0, 0.0);
const S_SHIPWRECK_115: StructureConfig =
    StructureConfig::c(165745295, 16, 8, StructureType::Shipwreck, 0, 0.0);
// 1.13 起 feature 按类型分 salt
const S_DESERT_PYRAMID: StructureConfig =
    StructureConfig::c(14357617, 32, 24, StructureType::DesertPyramid, 0, 0.0);
const S_IGLOO: StructureConfig =
    StructureConfig::c(14357618, 32, 24, StructureType::Igloo, 0, 0.0);
const S_JUNGLE_TEMPLE: StructureConfig =
    StructureConfig::c(14357619, 32, 24, StructureType::JungleTemple, 0, 0.0);
const S_SWAMP_HUT: StructureConfig =
    StructureConfig::c(14357620, 32, 24, StructureType::SwampHut, 0, 0.0);
const S_OUTPOST: StructureConfig =
    StructureConfig::c(165745296, 32, 24, StructureType::Outpost, 0, 0.0);
const S_VILLAGE_117: StructureConfig =
    StructureConfig::c(10387312, 32, 24, StructureType::Village, 0, 0.0);
const S_VILLAGE: StructureConfig =
    StructureConfig::c(10387312, 34, 26, StructureType::Village, 0, 0.0);
const S_OCEAN_RUIN: StructureConfig =
    StructureConfig::c(14357621, 20, 12, StructureType::OceanRuin, 0, 0.0);
const S_SHIPWRECK: StructureConfig =
    StructureConfig::c(165745295, 24, 20, StructureType::Shipwreck, 0, 0.0);
const S_MONUMENT: StructureConfig =
    StructureConfig::c(10387313, 32, 27, StructureType::Monument, 0, 0.0);
const S_MANSION: StructureConfig =
    StructureConfig::c(10387319, 80, 60, StructureType::Mansion, 0, 0.0);
const S_RUINED_PORTAL: StructureConfig =
    StructureConfig::c(34222645, 40, 25, StructureType::RuinedPortal, 0, 0.0);
// 注意：C 中此配置 structType 记为 Ruined_Portal（非 _N）
const S_RUINED_PORTAL_N: StructureConfig =
    StructureConfig::c(34222645, 40, 25, StructureType::RuinedPortal, DIM_NETHER, 0.0);
const S_RUINED_PORTAL_N_117: StructureConfig =
    StructureConfig::c(34222645, 25, 15, StructureType::RuinedPortalN, DIM_NETHER, 0.0);
const S_ANCIENT_CITY: StructureConfig =
    StructureConfig::c(20083232, 24, 16, StructureType::AncientCity, 0, 0.0);
const S_TRAIL_RUINS: StructureConfig =
    StructureConfig::c(83469867, 34, 26, StructureType::TrailRuins, 0, 0.0);
const S_TRIAL_CHAMBERS: StructureConfig =
    StructureConfig::c(94251327, 34, 22, StructureType::TrialChambers, 0, 0.0);
const S_TREASURE: StructureConfig =
    StructureConfig::c(10387320, 1, 1, StructureType::Treasure, 0, 0.0);
const S_MINESHAFT: StructureConfig =
    StructureConfig::c(0, 1, 1, StructureType::Mineshaft, 0, 0.0);
const S_DESERT_WELL_115: StructureConfig =
    StructureConfig::c(30010, 1, 1, StructureType::DesertWell, 0, 1.0 / 1000.0);
const S_DESERT_WELL_117: StructureConfig =
    StructureConfig::c(40013, 1, 1, StructureType::DesertWell, 0, 1.0 / 1000.0);
const S_DESERT_WELL: StructureConfig =
    StructureConfig::c(40002, 1, 1, StructureType::DesertWell, 0, 1.0 / 1000.0);
const S_GEODE_117: StructureConfig =
    StructureConfig::c(20000, 1, 1, StructureType::Geode, 0, 1.0 / 24.0);
const S_GEODE: StructureConfig =
    StructureConfig::c(20002, 1, 1, StructureType::Geode, 0, 1.0 / 24.0);
// 下界与末地结构
const S_FORTRESS_115: StructureConfig =
    StructureConfig::c(0, 16, 8, StructureType::Fortress, DIM_NETHER, 0.0);
const S_FORTRESS: StructureConfig =
    StructureConfig::c(30084232, 27, 23, StructureType::Fortress, DIM_NETHER, 0.0);
const S_BASTION: StructureConfig =
    StructureConfig::c(30084232, 27, 23, StructureType::Bastion, DIM_NETHER, 0.0);
const S_END_CITY: StructureConfig =
    StructureConfig::c(10387313, 20, 9, StructureType::EndCity, DIM_END, 0.0);
// 散落的回归折跃门
const S_END_GATEWAY_115: StructureConfig =
    StructureConfig::c(30000, 1, 1, StructureType::EndGateway, DIM_END, 700.0);
const S_END_GATEWAY_116: StructureConfig =
    StructureConfig::c(40013, 1, 1, StructureType::EndGateway, DIM_END, 700.0);
const S_END_GATEWAY_117: StructureConfig =
    StructureConfig::c(40013, 1, 1, StructureType::EndGateway, DIM_END, 1.0 / 700.0);
const S_END_GATEWAY: StructureConfig =
    StructureConfig::c(40000, 1, 1, StructureType::EndGateway, DIM_END, 1.0 / 700.0);
const S_END_ISLAND_116: StructureConfig =
    StructureConfig::c(0, 1, 1, StructureType::EndIsland, DIM_END, 14.0);
const S_END_ISLAND: StructureConfig =
    StructureConfig::c(0, 1, 1, StructureType::EndIsland, DIM_END, 1.0 / 14.0);

/// `getStructureConfig`：取指定版本下某结构类型的配置。
///
/// 该版本不支持此结构时返回 `None`（C 返回 0 且把配置清零）。
///
/// 注意：本库的版本下界是 1.7（cubiomes 的 `MC_1_7`），C 中涉及
/// `MC_B1_8`/`MC_1_3`/`MC_1_4` 等更早版本的条件在本库里恒为真，已按此化简
/// 并在相应分支注释说明。
pub fn get_config(stype: StructureType, mc: McVersion) -> Option<StructureConfig> {
    use StructureType::*;
    let conf = match stype {
        Feature => {
            if mc > McVersion::V1_12 {
                return None;
            }
            S_FEATURE
        }
        // mc >= MC_1_3 恒真（本库最旧为 1.7）
        DesertPyramid => {
            if mc <= McVersion::V1_12 {
                S_DESERT_PYRAMID_112
            } else {
                S_DESERT_PYRAMID
            }
        }
        JungleTemple => {
            if mc <= McVersion::V1_12 {
                S_JUNGLE_TEMPLE_112
            } else {
                S_JUNGLE_TEMPLE
            }
        }
        // mc >= MC_1_4 恒真
        SwampHut => {
            if mc <= McVersion::V1_12 {
                S_SWAMP_HUT_112
            } else {
                S_SWAMP_HUT
            }
        }
        Igloo => {
            if mc < McVersion::V1_9 {
                return None;
            }
            if mc <= McVersion::V1_12 {
                S_IGLOO_112
            } else {
                S_IGLOO
            }
        }
        // mc >= MC_B1_8 恒真
        Village => {
            if mc <= McVersion::V1_17 {
                S_VILLAGE_117
            } else {
                S_VILLAGE
            }
        }
        OceanRuin => {
            if mc < McVersion::V1_13 {
                return None;
            }
            if mc <= McVersion::V1_15 {
                S_OCEAN_RUIN_115
            } else {
                S_OCEAN_RUIN
            }
        }
        Shipwreck => {
            if mc < McVersion::V1_13 {
                return None;
            }
            if mc <= McVersion::V1_15 {
                S_SHIPWRECK_115
            } else {
                S_SHIPWRECK
            }
        }
        RuinedPortal => {
            if mc < McVersion::V1_16_1 {
                return None;
            }
            S_RUINED_PORTAL
        }
        RuinedPortalN => {
            if mc < McVersion::V1_16_1 {
                return None;
            }
            if mc <= McVersion::V1_17 {
                S_RUINED_PORTAL_N_117
            } else {
                S_RUINED_PORTAL_N
            }
        }
        Monument => {
            if mc < McVersion::V1_8 {
                return None;
            }
            S_MONUMENT
        }
        EndCity => {
            if mc < McVersion::V1_9 {
                return None;
            }
            S_END_CITY
        }
        Mansion => {
            if mc < McVersion::V1_11 {
                return None;
            }
            S_MANSION
        }
        Outpost => {
            if mc < McVersion::V1_14 {
                return None;
            }
            S_OUTPOST
        }
        AncientCity => {
            if mc < McVersion::V1_19_2 {
                return None;
            }
            S_ANCIENT_CITY
        }
        Treasure => {
            if mc < McVersion::V1_13 {
                return None;
            }
            S_TREASURE
        }
        // mc >= MC_B1_8 恒真
        Mineshaft => S_MINESHAFT,
        // mc >= MC_1_0 恒真
        Fortress => {
            if mc <= McVersion::V1_15 {
                S_FORTRESS_115
            } else {
                S_FORTRESS
            }
        }
        Bastion => {
            if mc < McVersion::V1_16_1 {
                return None;
            }
            S_BASTION
        }
        EndGateway => {
            // 1.11/1.12 的折跃门使用了经过方块填充的随机源，难以预测，C 不支持
            if mc < McVersion::V1_13 {
                return None;
            }
            if mc <= McVersion::V1_15 {
                S_END_GATEWAY_115
            } else if mc <= McVersion::V1_16 {
                S_END_GATEWAY_116
            } else if mc <= McVersion::V1_17 {
                S_END_GATEWAY_117
            } else {
                S_END_GATEWAY
            }
        }
        EndIsland => {
            // 装饰性 feature 仅支持 1.13+
            if mc < McVersion::V1_13 {
                return None;
            }
            if mc <= McVersion::V1_16 {
                S_END_ISLAND_116
            } else {
                S_END_ISLAND
            }
        }
        DesertWell => {
            // 1.2 引入，但装饰性 feature 仅支持 1.13+
            if mc < McVersion::V1_13 {
                return None;
            }
            if mc <= McVersion::V1_15 {
                S_DESERT_WELL_115
            } else if mc <= McVersion::V1_17 {
                S_DESERT_WELL_117
            } else {
                S_DESERT_WELL
            }
        }
        Geode => {
            if mc < McVersion::V1_17 {
                return None;
            }
            if mc <= McVersion::V1_17 {
                S_GEODE_117
            } else {
                S_GEODE
            }
        }
        TrailRuins => {
            if mc < McVersion::V1_20 {
                return None;
            }
            S_TRAIL_RUINS
        }
        TrialChambers => {
            if mc < McVersion::V1_21_1 {
                return None;
            }
            S_TRIAL_CHAMBERS
        }
    };
    Some(conf)
}
