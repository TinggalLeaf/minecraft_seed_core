//! 结构可行性检查：移植 cubiomes `finders.c` 的 `isViableFeatureBiome`、
//! `areBiomesViable`、`isViableStructurePos`，以及 `biomes.c` 的
//! `biomeExists`/`isOverworld`（本库版本范围内）。
//!
//! ## 粗层剪枝（`mapViableBiome`/`mapViableShore`）
//!
//! C 在 `isViableStructurePos`（1.7–1.17 主世界）检查前把
//! `L_BIOME_256`/`L_SHORE_16` 两层的 `getMap` 临时换成带提前终止的版本：
//! 若粗层区域中找不到任何目标群系，getMap 返回 err 并沿层链传播，
//! 本次群系查询整体作废（`getBiomeAt` 返回 `none` = -1）。
//! **这不是纯优化**：粗层（1:256/1:16）无目标群系而细层（含河流/海岸等
//! 后期层）恰好有的位置，C 会判不可行。本实现通过
//! [`crate::generator::layers::LayerStack::viable_filter`] 复刻该行为：
//! 层函数照常计算，但在 `get_map` 出口对这两层做相同的扫描，命中剪枝时
//! 置失败标记，查询入口据此返回 `None`/`-1`。
//!
//! ## 其它差异
//!
//! - C 对不支持的结构类型会 `exit(1)`；这里返回 `false`/`0` 并在文档注明。

use crate::biome::{is_deep_ocean, is_mesa, is_oceanic, BiomeId};
use crate::generator::layers::LayerId;
use crate::generator::{Generator, Range};
use crate::version::{Dimension, McVersion};

use super::config::{get_config, StructureType};
use super::region::{chunk_generate_rnd, get_feature_pos, get_structure_pos, set_attempt_seed};
use super::variant::get_variant;

/// C `floordiv`：向下取整除法（`b > 0`）。
#[inline]
pub(crate) fn floordiv(a: i32, b: i32) -> i32 {
    let q = a / b;
    if a % b != 0 && ((a ^ b) < 0) {
        q - 1
    } else {
        q
    }
}

/// C `idSetTest`：群系 ID 集合（低 64 位表示 id 0–63，高位表示 128–191）。
#[inline]
pub(crate) fn id_set_test(m_low: u64, m_mut: u64, id: i32) -> bool {
    match id & !0x3f {
        0 => m_low & (1u64 << id) != 0,
        128 => m_mut & (1u64 << (id - 128)) != 0,
        _ => false,
    }
}

/// `biomeExists`：群系在该版本是否存在（全版本保真，含 Beta 与 1.0–1.6；
/// 实现集中在 [`BiomeId::exists_in`]）。
pub fn biome_exists(mc: McVersion, id: i32) -> bool {
    match BiomeId::from_i32(id) {
        Some(b) => b.exists_in(mc),
        Option::None => false,
    }
}
/// `isOverworld`：群系是否为主世界群系（含版本存在性检查）。
pub fn is_overworld(mc: McVersion, id: i32) -> bool {
    use BiomeId::*;
    if !biome_exists(mc, id) {
        return false;
    }
    if (SmallEndIslands as i32..=EndBarrens as i32).contains(&id)
        || (SoulSandValley as i32..=BasaltDeltas as i32).contains(&id)
    {
        return false;
    }
    match BiomeId::from_i32(id) {
        Some(NetherWastes | TheEnd) => false,
        Some(FrozenOcean) => mc <= McVersion::V1_6 || mc >= McVersion::V1_13,
        Some(MountainEdge) => mc <= McVersion::V1_6,
        Some(DeepWarmOcean | TheVoid) => false,
        Some(TallBirchForest) => !(McVersion::V1_9..=McVersion::V1_10).contains(&mc),
        Some(DripstoneCaves | LushCaves) => mc >= McVersion::V1_18,
        _ => true,
    }
}

/// C `isViableFeatureBiome` 中 Outpost（1.17- 落入）/Village 的公共部分。
fn viable_village_biome(mc: McVersion, biome_id: i32) -> bool {
    use BiomeId::*;
    let b = BiomeId::from_i32(biome_id);
    if matches!(b, Some(Plains | Desert | Savanna)) {
        return true;
    }
    if mc >= McVersion::V1_10 && b == Some(Taiga) {
        return true;
    }
    if mc >= McVersion::V1_14 && b == Some(SnowyTundra) {
        return true;
    }
    if mc >= McVersion::V1_18 && b == Some(Meadow) {
        return true;
    }
    false
}

/// `isViableFeatureBiome`：结构类型能否生成于给定群系。
///
/// C 对未实现的类型（`Feature`/`Geode`/`End_Island`）直接 `exit(1)`；
/// 这里返回 `false`。
pub fn is_viable_feature_biome(mc: McVersion, stype: StructureType, biome_id: i32) -> bool {
    use BiomeId::*;
    use StructureType::*;
    let b = BiomeId::from_i32(biome_id);
    match stype {
        DesertPyramid => matches!(b, Some(Desert | DesertHills)),

        JungleTemple => matches!(
            b,
            Some(Jungle | JungleHills | BambooJungle | BambooJungleHills)
        ),

        SwampHut => b == Some(Swamp),

        Igloo => {
            if mc <= McVersion::V1_8 {
                return false;
            }
            matches!(b, Some(SnowyTundra | SnowyTaiga | SnowySlopes))
        }

        OceanRuin => {
            if mc <= McVersion::V1_12 {
                return false;
            }
            is_oceanic(biome_id)
        }

        Shipwreck => {
            if mc <= McVersion::V1_12 {
                return false;
            }
            is_oceanic(biome_id) || matches!(b, Some(Beach | SnowyBeach))
        }

        RuinedPortal | RuinedPortalN => mc >= McVersion::V1_16_1,

        AncientCity => {
            if mc <= McVersion::V1_18 {
                return false;
            }
            b == Some(DeepDark)
        }

        TrailRuins => {
            if mc <= McVersion::V1_19 {
                return false;
            }
            matches!(
                b,
                Some(
                    Taiga | SnowyTaiga | GiantTreeTaiga // old_growth_pine_taiga
                    | GiantSpruceTaiga // old_growth_spruce_taiga
                    | TallBirchForest // old_growth_birch_forest
                    | Jungle
                )
            )
        }

        TrialChambers => {
            if mc <= McVersion::V1_20 {
                return false;
            }
            b != Some(DeepDark) && is_overworld(mc, biome_id)
        }

        Treasure => {
            if mc <= McVersion::V1_12 {
                return false;
            }
            matches!(b, Some(Beach | SnowyBeach))
        }

        Mineshaft => is_overworld(mc, biome_id),

        DesertWell => b == Some(Desert),

        Monument => {
            if mc <= McVersion::V1_7 {
                return false;
            }
            is_deep_ocean(biome_id)
        }

        Outpost => {
            if mc <= McVersion::V1_13 {
                return false;
            }
            if mc >= McVersion::V1_18 {
                return matches!(
                    b,
                    Some(
                        Desert | Plains | Savanna | SnowyTundra | Taiga | Meadow
                            | FrozenPeaks | JaggedPeaks | StonyPeaks | SnowySlopes | Grove
                            | CherryGrove
                    )
                );
            }
            viable_village_biome(mc, biome_id)
        }

        Village => viable_village_biome(mc, biome_id),

        Mansion => {
            if mc <= McVersion::V1_10 {
                return false;
            }
            matches!(b, Some(DarkForest | DarkForestHills))
        }

        Fortress => matches!(
            b,
            Some(NetherWastes | SoulSandValley | WarpedForest | CrimsonForest | BasaltDeltas)
        ),

        Bastion => {
            if mc <= McVersion::V1_15 {
                return false;
            }
            matches!(
                b,
                Some(NetherWastes | SoulSandValley | WarpedForest | CrimsonForest)
            )
        }

        EndCity => {
            if mc <= McVersion::V1_8 {
                return false;
            }
            matches!(b, Some(EndMidlands | EndHighlands))
        }

        EndGateway => {
            if mc <= McVersion::V1_12 {
                return false;
            }
            b == Some(EndHighlands)
        }

        // C 中 exit(1) 的未实现类型：Feature / Geode / End_Island
        _ => false,
    }
}

// 海底神殿所需群系（C 的 g_monument_biomes1/2）
const MONUMENT_BIOMES2: u64 = (1u64 << BiomeId::DeepFrozenOcean as i32)
    | (1u64 << BiomeId::DeepColdOcean as i32)
    | (1u64 << BiomeId::DeepOcean as i32)
    | (1u64 << BiomeId::DeepLukewarmOcean as i32)
    | (1u64 << BiomeId::DeepWarmOcean as i32);

const MONUMENT_BIOMES1: u64 = (1u64 << BiomeId::Ocean as i32)
    | (1u64 << BiomeId::DeepOcean as i32)
    | (1u64 << BiomeId::River as i32)
    | (1u64 << BiomeId::FrozenRiver as i32)
    | (1u64 << BiomeId::FrozenOcean as i32)
    | MONUMENT_BIOMES2
    | (1u64 << BiomeId::ColdOcean as i32)
    | (1u64 << BiomeId::LukewarmOcean as i32)
    | (1u64 << BiomeId::WarmOcean as i32);

/// `mapViableBiome` 的扫描部分：`L_BIOME_256` 粗层区域内是否存在该结构
/// 类型的目标群系（C 中找不到则返回 err 剪枝；未列出的类型不剪枝）。
pub(crate) fn viable_biome_area_ok(styp: StructureType, ids: &[i32]) -> bool {
    use StructureType::*;
    let hit = |id: i32| match styp {
        DesertPyramid | DesertWell => id == BiomeId::Desert as i32 || is_mesa(id),
        JungleTemple => id == BiomeId::Jungle as i32,
        SwampHut => id == BiomeId::Swamp as i32,
        Igloo => id == BiomeId::SnowyTundra as i32 || id == BiomeId::SnowyTaiga as i32,
        Treasure | OceanRuin | Shipwreck | Monument => is_oceanic(id),
        Mansion => id == BiomeId::DarkForest as i32,
        _ => true, // C 的 default：直接返回 0（不剪枝）
    };
    ids.iter().any(|&id| hit(id))
}

/// `mapViableShore` 的扫描部分：`L_SHORE_16` 区域内是否存在可行群系
/// （未列出的类型不剪枝）。
pub(crate) fn viable_shore_area_ok(styp: StructureType, mc: McVersion, ids: &[i32]) -> bool {
    use StructureType::*;
    match styp {
        DesertPyramid | JungleTemple | SwampHut | Igloo | OceanRuin | Shipwreck | Village
        | Monument | Mansion | Treasure | DesertWell => {
            ids.iter().any(|&id| is_viable_feature_biome(mc, styp, id))
        }
        _ => true,
    }
}

/// `areBiomesViable`：检查以 `(x, z)` 为中心、半径 `rad`（方块）的正方形
/// 内（1:4 采样）所有群系都在 `valid_b`/`valid_m` 集合中。
///
/// `approx >= 1` 时只检查四个角（本库调用方均传 0）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn are_biomes_viable(
    g: &Generator,
    x: i32,
    y: i32,
    z: i32,
    rad: i32,
    valid_b: u64,
    valid_m: u64,
    approx: i32,
) -> bool {
    let x1 = (x - rad) >> 2;
    let x2 = (x + rad) >> 2;
    let sx = x2 - x1 + 1;
    let z1 = (z - rad) >> 2;
    let z2 = (z + rad) >> 2;
    let sz = z2 - z1 + 1;

    // 1.18+ 会按 y 构成立体检查，但本函数只用于海底神殿（海洋/河流），
    // 检查最低 y 的洞穴即可（译自 C 注释）。
    let y = (y - rad) >> 2;

    // 四角
    let corners = [(x1, z1), (x2, z2), (x1, z2), (x2, z1)];
    for &(cx, cz) in &corners {
        let id = match g.viable_gen_biomes(Range::new(4, cx, cz, 1, 1).with_y(y, 1)) {
            // 粗层剪枝（C: getBiomeAt 返回 none）
            None => return false,
            Some(v) => v[0] as i32,
        };
        if !id_set_test(valid_b, valid_m, id) {
            return false;
        }
    }
    if approx >= 1 {
        return true;
    }

    if g.version() >= McVersion::V1_18 {
        let bn = g.biome_noise().expect("1.18+ 主世界应有 BiomeNoise");
        for i in 0..sx {
            // dat 缓存每行重置（与 C 一致；仅为加速，结果与全查找相同）
            let mut dat = 0u64;
            for j in 0..sz {
                let (_, id) = crate::generator::v1_18::sample_biome_noise(
                    bn,
                    x1 + i,
                    y,
                    z1 + j,
                    Some(&mut dat),
                    0,
                );
                let id = id as i32;
                if id < 0 || !id_set_test(valid_b, valid_m, id) {
                    return false;
                }
            }
        }
    } else {
        let ids = match g.viable_gen_biomes(Range::new(4, x1, z1, sx, sz).with_y(y, 1)) {
            // genBiomes 的 err 传播（C: goto L_no）
            None => return false,
            Some(v) => v,
        };
        for id in ids {
            let id = id as i32;
            if id < 0 || !id_set_test(valid_b, valid_m, id) {
                return false;
            }
        }
    }
    true
}

/// `isViableStructurePos`：检查结构能否在给定方块坐标实际生成（群系等
/// 约束）。坐标由 [`get_structure_pos`] 得到。
///
/// 返回 0 表示不可行；非 0 表示可行（部分类型返回可行的群系 ID，与 C
/// 一致）。`flags` 为可选的结构特定信息（如村庄的群系变体，0 表示不限）。
///
/// 生成器须已按正确版本/维度/种子初始化。C 会对生成器做临时修改并在返回
/// 前恢复；本实现只读。
///
/// # Panics
///
/// Beta 1.7 及更早的主世界：C 的 `isViableStructurePos` 对 beta 只做了
/// 半成品支持（引用了不存在的分层群系源），这里同样不可用——
/// `set_viable_filter` 会因没有 LayerStack 而 panic。
pub fn is_viable_structure_pos(
    stype: StructureType,
    g: &Generator,
    x: i32,
    z: i32,
    flags: u32,
) -> i32 {
    use StructureType::*;
    let mc = g.version();

    // C 用 int64 计算 chunk 坐标与采样点，再截断为 int；坐标范围内等价
    let chunk_x = (x >> 4) as i64;
    let chunk_z = (z >> 4) as i64;

    match g.dim().expect("Generator: 先调用 with_seed") {
        Dimension::Nether => {
            if stype == Fortress && mc <= McVersion::V1_17 {
                return 1;
            }
            if mc <= McVersion::V1_15 {
                return 0;
            }
            if stype == RuinedPortalN {
                return 1;
            }
            if stype == Fortress {
                // 1.18+ 要塞生成在堡垒遗迹不生成的位置
                let sc = match get_config(Fortress, mc) {
                    Some(c) => c,
                    None => return 0,
                };
                let rx = floordiv(x, sc.region_size << 4);
                let rz = floordiv(z, sc.region_size << 4);
                if get_structure_pos(Bastion, mc, g.seed(), rx, rz).is_none() {
                    return 1;
                }
                return i32::from(is_viable_structure_pos(Bastion, g, x, z, flags) == 0);
            }
            let mut sample_y = 0;
            let (sample_x, sample_z);
            if mc >= McVersion::V1_18 && stype == Bastion {
                let sv = get_variant(Bastion, mc, g.seed(), x, z, -1).expect("Bastion variant");
                sample_x = (((chunk_x * 32 + 2 * sv.x as i64 + sv.sx as i64 - 1) / 2) >> 2) as i32;
                sample_z = (((chunk_z * 32 + 2 * sv.z as i64 + sv.sz as i64 - 1) / 2) >> 2) as i32;
                if mc >= McVersion::V1_19_2 {
                    sample_y = 33 >> 2; // 下界群系实际上不随 y 变化
                }
            } else {
                sample_x = (chunk_x * 4 + 2) as i32;
                sample_z = (chunk_z * 4 + 2) as i32;
            }
            let id =
                g.gen_biomes(Range::new(4, sample_x, sample_z, 1, 1).with_y(sample_y, 1))[0];
            return i32::from(is_viable_feature_biome(mc, stype, id as i32));
        }
        Dimension::End => {
            match stype {
                EndCity => {
                    if mc <= McVersion::V1_8 {
                        return 0;
                    }
                }
                EndGateway => {
                    if mc <= McVersion::V1_12 {
                        return 0;
                    }
                }
                _ => return 0,
            }
            // 末地群系按区块（1:16）划分；1.15 前的 voronoi 对末地城无影响，
            // 因为检查点在区块中心附近（译自 C 注释）
            let id = g.gen_biomes(Range::new(16, chunk_x as i32, chunk_z as i32, 1, 1))[0];
            return if is_viable_feature_biome(mc, stype, id as i32) {
                id as i32
            } else {
                0
            };
        }
        Dimension::Overworld => {}
    }

    // ---- 主世界 ----
    // C 在 1.7–1.17 下临时替换 L_BIOME_256/L_SHORE_16 的 getMap 为剪枝版本
    // （mapViableBiome/mapViableShore），返回前恢复；嵌套调用
    // （Outpost→Village）以内层过滤器为准，返回时恢复外层。
    if mc <= McVersion::V1_17 {
        let prev = g.set_viable_filter(Some((stype, mc)));
        let r = overworld_viable_pos(stype, g, x, z, flags, chunk_x, chunk_z);
        g.set_viable_filter(prev);
        r
    } else {
        overworld_viable_pos(stype, g, x, z, flags, chunk_x, chunk_z)
    }
}

/// `isViableStructurePos` 的主世界部分（见 [`is_viable_structure_pos`]）。
#[allow(clippy::too_many_arguments)]
fn overworld_viable_pos(
    stype: StructureType,
    g: &Generator,
    x: i32,
    z: i32,
    flags: u32,
    chunk_x: i64,
    chunk_z: i64,
) -> i32 {
    use StructureType::*;
    let mc = g.version();
    match stype {
        // L_feature：三神殿 + 雪屋/海洋废墟/沉船/宝藏/古迹废墟共用
        DesertPyramid | JungleTemple | SwampHut | Igloo | OceanRuin | Shipwreck | Treasure
        | TrailRuins => {
            match stype {
                TrailRuins if mc <= McVersion::V1_19 => return 0,
                OceanRuin | Shipwreck | Treasure if mc <= McVersion::V1_12 => return 0,
                Igloo if mc <= McVersion::V1_8 => return 0,
                _ => {}
            }
            let id = if mc <= McVersion::V1_15 {
                let sample_x = (chunk_x * 16 + 9) as i32;
                let sample_z = (chunk_z * 16 + 9) as i32;
                g.viable_layered_biome_at(LayerId::Voronoi1, sample_x, sample_z)
            } else if mc <= McVersion::V1_17 {
                let sample_x = (chunk_x * 4 + 2) as i32;
                let sample_z = (chunk_z * 4 + 2) as i32;
                g.viable_layered_biome_at(LayerId::RiverMix4, sample_x, sample_z)
            } else {
                let sample_x = (chunk_x * 4 + 2) as i32;
                let sample_z = (chunk_z * 4 + 2) as i32;
                g.get_biome(sample_x, 319 >> 2, sample_z) as i32
            };
            i32::from(id >= 0 && is_viable_feature_biome(mc, stype, id))
        }

        DesertWell => {
            let id = if mc <= McVersion::V1_15 {
                g.viable_layered_biome_at(LayerId::Voronoi1, x, z)
            } else if mc <= McVersion::V1_17 {
                g.viable_layered_biome_at(LayerId::RiverMix4, x >> 2, z >> 2)
            } else {
                g.get_biome(x >> 2, 319 >> 2, z >> 2) as i32
            };
            i32::from(id >= 0 && is_viable_feature_biome(mc, stype, id))
        }

        Village => {
            if mc <= McVersion::V1_17 {
                let id = if mc == McVersion::V1_15 {
                    // 仅 1.15 的村庄与其它结构共用同一种群系检查
                    g.viable_layered_biome_at(
                        LayerId::Voronoi1,
                        (chunk_x * 16 + 9) as i32,
                        (chunk_z * 16 + 9) as i32,
                    )
                } else {
                    g.viable_layered_biome_at(
                        LayerId::RiverMix4,
                        (chunk_x * 4 + 2) as i32,
                        (chunk_z * 4 + 2) as i32,
                    )
                };
                if id < 0 || !is_viable_feature_biome(mc, stype, id) {
                    return 0;
                }
                if flags != 0 && id as u32 != flags {
                    return 0;
                }
                let mut id = id;
                if mc <= McVersion::V1_9 {
                    // 1.10 之前的村庄不会蔓延进非法群系，会在起始区块
                    // (2,2) 的首次检查失败
                    id = g.viable_layered_biome_at(
                        LayerId::Voronoi1,
                        (chunk_x * 16 + 2) as i32,
                        (chunk_z * 16 + 2) as i32,
                    );
                    if id < 0 || !is_viable_feature_biome(mc, stype, id) {
                        return 0;
                    }
                }
                id // 返回可行群系，供进一步分析
            } else {
                // 1.18 起村庄类型分开检查
                let vv = [
                    BiomeId::Plains as i32,
                    BiomeId::Desert as i32,
                    BiomeId::Savanna as i32,
                    BiomeId::Taiga as i32,
                    BiomeId::SnowyTundra as i32,
                ];
                for &v in &vv {
                    if flags != 0 && flags != v as u32 {
                        continue;
                    }
                    let sv = match get_variant(Village, mc, g.seed(), x, z, v) {
                        Some(sv) => sv,
                        None => continue,
                    };
                    let sample_x =
                        (((chunk_x * 32 + 2 * sv.x as i64 + sv.sx as i64 - 1) / 2) >> 2) as i32;
                    let sample_z =
                        (((chunk_z * 32 + 2 * sv.z as i64 + sv.sz as i64 - 1) / 2) >> 2) as i32;
                    let sample_y = 319 >> 2;
                    let id = g.get_biome(sample_x, sample_y, sample_z) as i32;
                    if id == v || (id == BiomeId::Meadow as i32 && v == BiomeId::Plains as i32)
                    {
                        return v;
                    }
                }
                0
            }
        }

        Outpost => {
            if mc <= McVersion::V1_13 {
                return 0;
            }
            let mut s = g.seed();
            let mut rng = set_attempt_seed(&mut s, chunk_x as i32, chunk_z as i32);
            if rng.next_int_bound(5) != 0 {
                return 0;
            }
            // 检查 10 区块范围内有无村庄
            let vilconf = match get_config(Village, mc) {
                Some(c) => c,
                None => return 0,
            };
            let (cx0, cx1) = (chunk_x as i32 - 10, chunk_x as i32 + 10);
            let (cz0, cz1) = (chunk_z as i32 - 10, chunk_z as i32 + 10);
            let (rx0, rx1) = (
                floordiv(cx0, vilconf.region_size),
                floordiv(cx1, vilconf.region_size),
            );
            let (rz0, rz1) = (
                floordiv(cz0, vilconf.region_size),
                floordiv(cz1, vilconf.region_size),
            );
            for rz in rz0..=rz1 {
                for rx in rx0..=rx1 {
                    let p = get_feature_pos(&vilconf, g.seed(), rx, rz);
                    let (cx, cz) = (p.x >> 4, p.z >> 4);
                    if (cx0..=cx1).contains(&cx) && (cz0..=cz1).contains(&cz) {
                        if mc >= McVersion::V1_16_1 {
                            return 0;
                        }
                        if is_viable_structure_pos(Village, g, p.x, p.z, 0) != 0 {
                            return 0;
                        }
                    }
                }
            }
            let id = if mc >= McVersion::V1_18 {
                let mut rng = chunk_generate_rnd(g.seed(), chunk_x as i32, chunk_z as i32);
                let (dx, dz): (i64, i64) = match rng.next_int_bound(4) {
                    0 => (15, 15),
                    1 => (-15, 15),
                    2 => (-15, -15),
                    _ => (15, -15),
                };
                let sample_x = (((chunk_x * 32 + dx) / 2) >> 2) as i32;
                let sample_z = (((chunk_z * 32 + dz) / 2) >> 2) as i32;
                g.get_biome(sample_x, 319 >> 2, sample_z) as i32
            } else if mc >= McVersion::V1_16_1 {
                g.viable_layered_biome_at(
                    LayerId::RiverMix4,
                    (chunk_x * 4 + 2) as i32,
                    (chunk_z * 4 + 2) as i32,
                )
            } else {
                g.viable_layered_biome_at(
                    LayerId::Voronoi1,
                    (chunk_x * 16 + 9) as i32,
                    (chunk_z * 16 + 9) as i32,
                )
            };
            i32::from(id >= 0 && is_viable_feature_biome(mc, stype, id))
        }

        Monument => {
            if mc <= McVersion::V1_7 {
                return 0;
            }
            if mc == McVersion::V1_8 {
                // 1.8 的海底神殿只要求单个深海方块
                let id = g.viable_layered_biome_at(
                    LayerId::Voronoi1,
                    (chunk_x * 16 + 8) as i32,
                    (chunk_z * 16 + 8) as i32,
                );
                if id < 0 || !is_deep_ocean(id) {
                    return 0;
                }
            } else if mc <= McVersion::V1_17 {
                // 海底神殿需要两次含海洋层分支的可行性检查，
                // 值得先粗查是否存在深海（译自 C 注释）
                let id = g.viable_layered_biome_at(LayerId::Shore16, chunk_x as i32, chunk_z as i32);
                if id < 0 || !is_deep_ocean(id) {
                    return 0;
                }
            }
            let sample_x = (chunk_x * 16 + 8) as i32;
            let sample_z = (chunk_z * 16 + 8) as i32;
            if (McVersion::V1_9..=McVersion::V1_17).contains(&mc) {
                // 深海中心检查
                if !are_biomes_viable(g, sample_x, 63, sample_z, 16, MONUMENT_BIOMES2, 0, 0) {
                    return 0;
                }
            } else if mc >= McVersion::V1_18 {
                // 在海床高度检查——以 y = 36 近似（译自 C 注释）
                let id = g.get_biome(sample_x >> 2, 36 >> 2, sample_z >> 2) as i32;
                if !is_deep_ocean(id) {
                    return 0;
                }
            }
            if are_biomes_viable(g, sample_x, 63, sample_z, 29, MONUMENT_BIOMES1, 0, 0) {
                1
            } else {
                0
            }
        }

        Mansion => {
            if mc <= McVersion::V1_10 {
                return 0;
            }
            if mc <= McVersion::V1_17 {
                let sample_x = (chunk_x * 16 + 8) as i32;
                let sample_z = (chunk_z * 16 + 8) as i32;
                let b = 1u64 << BiomeId::DarkForest as i32;
                let m = 1u64 << (BiomeId::DarkForestHills as i32 - 128);
                if !are_biomes_viable(g, sample_x, 0, sample_z, 32, b, m, 0) {
                    return 0;
                }
            } else {
                // 1.18 会取结构四角的最小地表高度（结构带旋转），要求 >= 60；
                // 群系检查在该高度的中心位置进行。
                // TODO(cubiomes 同样未做)：取地表高度
                let sample_x = (chunk_x * 16 + 7) as i32;
                let sample_z = (chunk_z * 16 + 7) as i32;
                let id = g.get_biome(sample_x >> 2, 319 >> 2, sample_z >> 2) as i32;
                if id < 0 || !is_viable_feature_biome(mc, stype, id) {
                    return 0;
                }
            }
            1
        }

        RuinedPortal | RuinedPortalN => i32::from(mc > McVersion::V1_15),

        Geode => i32::from(mc > McVersion::V1_16),

        AncientCity | TrialChambers => {
            if stype == AncientCity && mc <= McVersion::V1_18 {
                return 0;
            }
            if stype == TrialChambers && mc <= McVersion::V1_20 {
                return 0;
            }
            // L_jigsaw
            let sv = match get_variant(stype, mc, g.seed(), x, z, -1) {
                Some(sv) => sv,
                None => return 0,
            };
            let sample_x = (((chunk_x * 32 + 2 * sv.x as i64 + sv.sx as i64 - 1) / 2) >> 2) as i32;
            let sample_z = (((chunk_z * 32 + 2 * sv.z as i64 + sv.sz as i64 - 1) / 2) >> 2) as i32;
            let sample_y = sv.y >> 2;
            let id = g.get_biome(sample_x, sample_y, sample_z) as i32;
            i32::from(id >= 0 && is_viable_feature_biome(mc, stype, id))
        }

        Mineshaft => 1,

        // C 的 default 分支：未知类型/维度组合打印错误并视为不可行
        _ => 0,
    }
}

// ============================================================================
// 地形级可行性（finders.c 的 isViableStructureTerrain / isViableEndCityTerrain
// / isEndChunkEmpty）
// ============================================================================

use crate::generator::end::{get_surface_height, END_LOWER_DROP, END_UPPER_DROP};
use crate::noise::surface::SurfaceNoise;


/// `isEndChunkEmpty` 的反演钳制表：`(30*(1-l)/l + 3000*(1-u)) / u` 的逆，
/// 用于把"噪声是否可能超过阈值"转换成 `sampleSurfaceNoiseBetween` 的
/// 早退区间（译自 C 注释）。
const END_INVERSE_DROP: [f64; 19] = [
    1e9, 1e9, 180.0, 75.0, 40.0, 22.5, 12.0, 5.0, // 0-7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, // 8-14
    1000.0 / 21.0, 3000.0 / 31.0, 9000.0 / 61.0, 200.0, // 15-18
];

/// `isViableStructureTerrain`：结构的地形级可行性检查（1.18+ 主世界）。
///
/// 用 depth 气候参数（≈地表高度，0.5 约等于海平面）检查结构 footprint
/// 的四角：任一角 `depth < 0.48` 则不可行。只覆盖沙漠神殿、丛林神庙与
/// 林地府邸（含随机朝向决定的 footprint 方向）；其余结构类型以及
/// 1.17- 恒为 `true`（与 C 一致）。
///
/// 坐标 `(x, z)` 为方块级（通常来自 [`get_structure_pos`]）。
pub fn is_viable_structure_terrain(stype: StructureType, g: &Generator, x: i32, z: i32) -> bool {
    let mc = g.version();
    if mc <= McVersion::V1_17 {
        return true;
    }
    let (x, z, sx, sz) = match stype {
        StructureType::DesertPyramid => (x, z, 21, 21),
        StructureType::JungleTemple => (x, z, 12, 15),
        StructureType::Mansion => {
            let cx = x >> 4;
            let cz = z >> 4;
            let mut rng = chunk_generate_rnd(g.seed(), cx, cz);
            let rot = rng.next_int_bound(4);
            let (mut sx, mut sz) = (5, 5);
            if rot == 0 {
                sx = -5;
            }
            if rot == 1 {
                sx = -5;
                sz = -5;
            }
            if rot == 2 {
                sz = -5;
            }
            (cx * 16 + 7, cz * 16 + 7, sx, sz)
        }
        _ => return true,
    };

    // 以 depth 参数近似地表高度（0.5 ≈ 海平面）
    let corners = [
        (x as f64 / 4.0, z as f64 / 4.0),
        ((x + sx) as f64 / 4.0, (z + sz) as f64 / 4.0),
        (x as f64 / 4.0, (z + sz) as f64 / 4.0),
        ((x + sx) as f64 / 4.0, z as f64 / 4.0),
    ];
    let bn = g.biome_noise().expect("1.18+ 主世界应有 BiomeNoise");
    for &(cx, cz) in &corners {
        let depth = bn.sample_depth(cx, cz);
        if depth < 0.48 {
            return false;
        }
    }
    true
}

/// `isViableEndCityTerrain`：末地城的地形级可行性。
///
/// 返回区块四角（含随机朝向决定的 5 方块偏移）的最小地表高度；`< 60`
/// 时返回 0（不可行），否则返回该高度（与 C 的返回值语义一致）。
///
/// `sn` 须为 `SurfaceNoise::new(Dimension::End, seed)`（末地维度下
/// [`Generator::surface_noise`] 等价）。
pub fn is_viable_end_city_terrain(
    g: &Generator,
    sn: &SurfaceNoise,
    block_x: i32,
    block_z: i32,
) -> i32 {
    const Y0: i32 = 15;
    const Y1: i32 = 18;
    const YN: usize = (Y1 - Y0 + 1) as usize;

    let en = g.end_noise().expect("isViableEndCityTerrain: 生成器须为末地维度");
    let chunk_x = block_x >> 4;
    let chunk_z = block_z >> 4;
    let block_x = chunk_x * 16 + 7;
    let block_z = chunk_z * 16 + 7;
    let cellx = block_x >> 3;
    let cellz = block_z >> 3;

    // ncol[i][j]：(cellx+i, cellz+j) 的噪声列
    let mut ncol = [[[0.0f64; YN]; 3]; 3];
    {
        en.sample_noise_column_end(sn, &mut ncol[0][0], cellx, cellz, Y0, Y1);
        en.sample_noise_column_end(sn, &mut ncol[0][1], cellx, cellz + 1, Y0, Y1);
        en.sample_noise_column_end(sn, &mut ncol[1][0], cellx + 1, cellz, Y0, Y1);
        en.sample_noise_column_end(sn, &mut ncol[1][1], cellx + 1, cellz + 1, Y0, Y1);

        let h00 = get_surface_height(
            &ncol[0][0], &ncol[0][1], &ncol[1][0], &ncol[1][1],
            Y0, Y1, 4,
            (block_x & 7) as f64 / 8.0, (block_z & 7) as f64 / 8.0,
        );

        let mut cs = if en.mc() <= McVersion::V1_18 {
            crate::rng::JavaRandom::new(
                (chunk_x as u64).wrapping_add((chunk_z as u64).wrapping_mul(10387313)) as i64,
            )
        } else {
            chunk_generate_rnd(g.seed(), chunk_x, chunk_z)
        };

        let (h01, h10, h11);
        match cs.next_int_bound(4) {
            0 => {
                // (++) 0
                en.sample_noise_column_end(sn, &mut ncol[0][2], cellx, cellz + 2, Y0, Y1);
                en.sample_noise_column_end(sn, &mut ncol[1][2], cellx + 1, cellz + 2, Y0, Y1);
                en.sample_noise_column_end(sn, &mut ncol[2][0], cellx + 2, cellz, Y0, Y1);
                en.sample_noise_column_end(sn, &mut ncol[2][1], cellx + 2, cellz + 1, Y0, Y1);
                en.sample_noise_column_end(sn, &mut ncol[2][2], cellx + 2, cellz + 2, Y0, Y1);
                h01 = get_surface_height(
                    &ncol[0][1], &ncol[0][2], &ncol[1][1], &ncol[1][2],
                    Y0, Y1, 4,
                    (block_x & 7) as f64 / 8.0, ((block_z + 5) & 7) as f64 / 8.0,
                );
                h10 = get_surface_height(
                    &ncol[1][0], &ncol[1][1], &ncol[2][0], &ncol[2][1],
                    Y0, Y1, 4,
                    ((block_x + 5) & 7) as f64 / 8.0, (block_z & 7) as f64 / 8.0,
                );
                h11 = get_surface_height(
                    &ncol[1][1], &ncol[1][2], &ncol[2][1], &ncol[2][2],
                    Y0, Y1, 4,
                    ((block_x + 5) & 7) as f64 / 8.0, ((block_z + 5) & 7) as f64 / 8.0,
                );
            }
            1 => {
                // (-+) 90
                en.sample_noise_column_end(sn, &mut ncol[0][2], cellx, cellz + 2, Y0, Y1);
                en.sample_noise_column_end(sn, &mut ncol[1][2], cellx + 1, cellz + 2, Y0, Y1);
                h01 = get_surface_height(
                    &ncol[0][1], &ncol[0][2], &ncol[1][1], &ncol[1][2],
                    Y0, Y1, 4,
                    (block_x & 7) as f64 / 8.0, ((block_z + 5) & 7) as f64 / 8.0,
                );
                h10 = get_surface_height(
                    &ncol[0][0], &ncol[0][1], &ncol[1][0], &ncol[1][1],
                    Y0, Y1, 4,
                    ((block_x - 5) & 7) as f64 / 8.0, (block_z & 7) as f64 / 8.0,
                );
                h11 = get_surface_height(
                    &ncol[0][1], &ncol[0][2], &ncol[1][1], &ncol[1][2],
                    Y0, Y1, 4,
                    ((block_x - 5) & 7) as f64 / 8.0, ((block_z + 5) & 7) as f64 / 8.0,
                );
            }
            2 => {
                // (--) 180
                h01 = get_surface_height(
                    &ncol[0][0], &ncol[0][1], &ncol[1][0], &ncol[1][1],
                    Y0, Y1, 4,
                    (block_x & 7) as f64 / 8.0, ((block_z - 5) & 7) as f64 / 8.0,
                );
                h10 = get_surface_height(
                    &ncol[0][0], &ncol[0][1], &ncol[1][0], &ncol[1][1],
                    Y0, Y1, 4,
                    ((block_x - 5) & 7) as f64 / 8.0, (block_z & 7) as f64 / 8.0,
                );
                h11 = get_surface_height(
                    &ncol[0][0], &ncol[0][1], &ncol[1][0], &ncol[1][1],
                    Y0, Y1, 4,
                    ((block_x - 5) & 7) as f64 / 8.0, ((block_z - 5) & 7) as f64 / 8.0,
                );
            }
            3 => {
                // (+-) 270
                en.sample_noise_column_end(sn, &mut ncol[2][0], cellx + 2, cellz, Y0, Y1);
                en.sample_noise_column_end(sn, &mut ncol[2][1], cellx + 2, cellz + 1, Y0, Y1);
                h01 = get_surface_height(
                    &ncol[0][0], &ncol[0][1], &ncol[1][0], &ncol[1][1],
                    Y0, Y1, 4,
                    (block_x & 7) as f64 / 8.0, ((block_z - 5) & 7) as f64 / 8.0,
                );
                h10 = get_surface_height(
                    &ncol[1][0], &ncol[1][1], &ncol[2][0], &ncol[2][1],
                    Y0, Y1, 4,
                    ((block_x + 5) & 7) as f64 / 8.0, (block_z & 7) as f64 / 8.0,
                );
                h11 = get_surface_height(
                    &ncol[1][0], &ncol[1][1], &ncol[2][0], &ncol[2][1],
                    Y0, Y1, 4,
                    ((block_x + 5) & 7) as f64 / 8.0, ((block_z - 5) & 7) as f64 / 8.0,
                );
            }
            _ => return 0, // 不可达（next_int_bound(4) ∈ 0..4）
        }
        let h = h00.min(h01).min(h10).min(h11);
        if h >= 60 { h } else { 0 }
    }
}

/// `isEndChunkEmpty`：末地 chunk 是否完全无陆地（小末地岛与地形噪声
/// 双重判定，含 `inverse_drop` 早退优化）。
///
/// 用于折跃门落点搜索等；返回 `true` 表示空区块。
pub fn is_end_chunk_empty(
    en: &crate::generator::EndNoise,
    sn: &SurfaceNoise,
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
) -> bool {
    let x = chunk_x * 2;
    let z = chunk_z * 2;
    let mut depth = [[0.0f64; 3]; 3];

    // 检查小末地岛是否落入该 chunk
    for j in -1..=1 {
        for i in -1..=1 {
            for isle in super::region::get_end_islands(en.mc(), seed, chunk_x + i, chunk_z + j) {
                if isle.x + isle.r <= chunk_x * 16 {
                    continue;
                }
                if isle.z + isle.r <= chunk_z * 16 {
                    continue;
                }
                if isle.x - isle.r > chunk_x * 16 + 15 {
                    continue;
                }
                if isle.z - isle.r > chunk_z * 16 + 15 {
                    continue;
                }
                let id = en.map_end_biome(isle.x >> 4, isle.z >> 4, 1, 1)[0];
                if id == BiomeId::SmallEndIslands {
                    return false;
                }
            }
        }
    }

    const EPS: f64 = 0.001;

    // 内部 depth 值是否已蕴含方块
    for (i, row) in depth.iter_mut().enumerate().take(2) {
        for (j, cell) in row.iter_mut().enumerate().take(2) {
            *cell = (en.get_end_height_noise(x + i as i32, z + j as i32, 0) - 8.0) as f64;
            for k in 8..=14 {
                let u = END_UPPER_DROP[k];
                let l = END_LOWER_DROP[k];
                let mut noise = *cell;
                let pivot = END_INVERSE_DROP[k] - noise;
                noise += sn.sample_between(
                    x + i as i32, k as i32, z + j as i32, pivot - EPS, pivot + EPS,
                );
                let noise = crate::noise::perlin::lerp(u, -3000.0, noise);
                let noise = crate::noise::perlin::lerp(l, -30.0, noise);
                if noise > 0.0 {
                    return false;
                }
            }
        }
    }

    // 邻接 chunk 边界的 depth
    for (i, row) in depth.iter_mut().enumerate() {
        row[2] = (en.get_end_height_noise(x + i as i32, z + 2, 0) - 8.0) as f64;
    }
    for (j, cell) in depth[2].iter_mut().enumerate().take(2) {
        *cell = (en.get_end_height_noise(x + 2, z + j as i32, 0) - 8.0) as f64;
    }

    // 若所有噪声值都不可能生成方块则为空
    let mut check_full = false;
    'outer: for (i, row) in depth.iter().enumerate() {
        for (j, &base) in row.iter().enumerate() {
            for k in 2..18 {
                let u = END_UPPER_DROP[k];
                let l = END_LOWER_DROP[k];
                let mut noise = base;
                let pivot = END_INVERSE_DROP[k] - noise;
                noise += sn.sample_between(
                    x + i as i32, k as i32, z + j as i32, pivot - EPS, pivot + EPS,
                );
                let noise = crate::noise::perlin::lerp(u, -3000.0, noise);
                let noise = crate::noise::perlin::lerp(l, -30.0, noise);
                if noise > 0.0 {
                    check_full = true;
                    break 'outer;
                }
            }
        }
    }
    if !check_full {
        return true;
    }

    let y = en.map_end_surface_height(sn, chunk_x * 16, chunk_z * 16, 16, 16, 1, 0);
    y.iter().all(|&v| v == 0.0)
}
