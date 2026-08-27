//! 结构实例变体：移植 cubiomes `finders.c` 的 `getVariant`。
//!
//! 计算结构实例的朝向、起始部件、包围盒尺寸等。`isViableStructurePos` 在
//! 1.18+ 对 Bastion/Village/Ancient_City/Trial_Chambers 的群系采样点
//! 依赖这里的包围盒，必须逐位一致。

use crate::biome::{get_category, BiomeId};
use crate::rng::{JavaRandom, Xoroshiro};
use crate::version::McVersion;

use super::config::{get_config, StructureType};
use super::region::{chunk_generate_rnd, get_population_seed};
use super::viability::is_viable_feature_biome;

/// 结构变体信息（对应 C `StructureVariant`；位域展平为 `bool`）。
///
/// C 中 `memset` 后置 `start = -1`、`biome = -1`、`y = 320`，见
/// [`StructureVariant::new`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructureVariant {
    pub abandoned: bool,   // 僵尸村庄
    pub giant: bool,       // 巨型传送门
    pub underground: bool, // 地下传送门
    pub airpocket: bool,   // 带空气泡的传送门
    pub basement: bool,    // 带地下室雪屋
    pub cracked: bool,     // 带裂缝紫晶洞
    pub size: i32,         // 紫晶洞尺寸 | 雪屋中段数量
    pub start: i32,        // 起始部件下标（-1 = 无）
    pub biome: i32,        // 群系变体（-1 = 无）
    pub rotation: i32,     // 0:0, 1:cw90, 2:cw180, 3:cw270
    pub mirror: bool,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub sx: i32,
    pub sy: i32,
    pub sz: i32,
}

impl StructureVariant {
    /// C `getVariant` 开头的初始化状态。
    fn new() -> Self {
        StructureVariant {
            abandoned: false,
            giant: false,
            underground: false,
            airpocket: false,
            basement: false,
            cracked: false,
            size: 0,
            start: -1,
            biome: -1,
            rotation: 0,
            mirror: false,
            x: 0,
            y: 320,
            z: 0,
            sx: 0,
            sy: 0,
            sz: 0,
        }
    }
}

impl Default for StructureVariant {
    fn default() -> Self {
        Self::new()
    }
}

/// `getVariant`：取结构实例的变体数据（仅支持部分结构类型）。
///
/// `x, z` 为结构所在区块的方块原点（`getStructurePos` 的输出）。
/// `biome_id` 为群系变体提示（村庄用），不需要时传 -1。
/// 不支持或不可生成时返回 `None`（C 返回 0）。
pub fn get_variant(
    stype: StructureType,
    mc: McVersion,
    seed: u64,
    x: i32,
    z: i32,
    biome_id: i32,
) -> Option<StructureVariant> {
    use StructureType::*;
    let mut rng = chunk_generate_rnd(seed, x >> 4, z >> 4);
    let mut r = StructureVariant::new();

    match stype {
        Village => {
            if mc <= McVersion::V1_9 {
                return None;
            }
            if !is_viable_feature_biome(mc, Village, biome_id) {
                return None;
            }
            if mc <= McVersion::V1_13 {
                rng.skip(if mc == McVersion::V1_13 { 10 } else { 11 });
                r.abandoned = rng.next_int_bound(50) == 0;
                return Some(r);
            }
            r.biome = biome_id;
            r.rotation = rng.next_int_bound(4);
            // (start, sx, sy, sz, abandoned)
            let (start, sx, sy, sz, abandoned): (i32, i32, i32, i32, bool) =
                match biome_id {
                    b if b == BiomeId::Meadow as i32 || b == BiomeId::Plains as i32 => {
                        if b == BiomeId::Meadow as i32 {
                            r.biome = BiomeId::Plains as i32;
                        }
                        let t = rng.next_int_bound(204);
                        if t < 50 {
                            (0, 9, 4, 9, false) // plains_fountain_01
                        } else if t < 100 {
                            (1, 10, 7, 10, false) // plains_meeting_point_1
                        } else if t < 150 {
                            (2, 8, 5, 15, false) // plains_meeting_point_2
                        } else if t < 200 {
                            (3, 11, 9, 11, false) // plains_meeting_point_3
                        } else if t < 201 {
                            (0, 9, 4, 9, true)
                        } else if t < 202 {
                            (1, 10, 7, 10, true)
                        } else if t < 203 {
                            (2, 8, 5, 15, true)
                        } else {
                            (3, 11, 9, 11, true)
                        }
                    }
                    b if b == BiomeId::Desert as i32 => {
                        let t = rng.next_int_bound(250);
                        if t < 98 {
                            (1, 17, 6, 9, false) // desert_meeting_point_1
                        } else if t < 196 {
                            (2, 12, 6, 12, false) // desert_meeting_point_2
                        } else if t < 245 {
                            (3, 15, 6, 15, false) // desert_meeting_point_3
                        } else if t < 247 {
                            (1, 17, 6, 9, true)
                        } else if t < 249 {
                            (2, 12, 6, 12, true)
                        } else {
                            (3, 15, 6, 15, true)
                        }
                    }
                    b if b == BiomeId::Savanna as i32 => {
                        let t = rng.next_int_bound(459);
                        if t < 100 {
                            (1, 14, 5, 12, false) // savanna_meeting_point_1
                        } else if t < 150 {
                            (2, 11, 6, 11, false) // savanna_meeting_point_2
                        } else if t < 300 {
                            (3, 9, 6, 11, false) // savanna_meeting_point_3
                        } else if t < 450 {
                            (4, 9, 6, 9, false) // savanna_meeting_point_4
                        } else if t < 452 {
                            (1, 14, 5, 12, true)
                        } else if t < 453 {
                            (2, 11, 6, 11, true)
                        } else if t < 456 {
                            (3, 9, 6, 11, true)
                        } else {
                            (4, 9, 6, 9, true)
                        }
                    }
                    b if b == BiomeId::Taiga as i32 => {
                        let t = rng.next_int_bound(100);
                        if t < 49 {
                            (1, 22, 3, 18, false) // taiga_meeting_point_1
                        } else if t < 98 {
                            (2, 9, 7, 9, false) // taiga_meeting_point_2
                        } else if t < 99 {
                            (1, 22, 3, 18, true)
                        } else {
                            (2, 9, 7, 9, true)
                        }
                    }
                    b if b == BiomeId::SnowyTundra as i32 => {
                        let t = rng.next_int_bound(306);
                        if t < 100 {
                            (1, 12, 8, 8, false) // snowy_meeting_point_1
                        } else if t < 150 {
                            (2, 11, 5, 9, false) // snowy_meeting_point_2
                        } else if t < 300 {
                            (3, 7, 7, 7, false) // snowy_meeting_point_3
                        } else if t < 302 {
                            (1, 12, 8, 8, true)
                        } else if t < 303 {
                            (2, 11, 5, 9, true)
                        } else {
                            (3, 7, 7, 7, true)
                        }
                    }
                    _ => return None,
                };
            r.start = start;
            r.abandoned = abandoned;
            rotate_village_bastion(&mut r, mc, x, z, sx, sy, sz);
            Some(r)
        }

        Bastion => {
            r.rotation = rng.next_int_bound(4);
            r.start = rng.next_int_bound(4);
            if mc == McVersion::V1_16_1 {
                // 仅 1.16.1 中两者互换
                std::mem::swap(&mut r.start, &mut r.rotation);
            }
            let (sx, sy, sz) = match r.start {
                0 => (46, 24, 46), // units/air_base
                1 => (30, 24, 48), // hoglin_stable/air_base
                2 => (38, 48, 38), // treasure/big_air_full
                _ => (16, 32, 32), // bridge/starting_pieces/entrance_base
            };
            rotate_village_bastion(&mut r, mc, x, z, sx, sy, sz);
            Some(r)
        }

        AncientCity => {
            r.rotation = rng.next_int_bound(4);
            r.start = 1 + rng.next_int_bound(3); // city_center_1..3
            let (sx, sy, sz) = (18, 31, 41);
            let (mut x, mut z) = (x, z);
            match r.rotation {
                0 => {
                    x = -(i32::from(x > 0));
                    z = -(i32::from(z > 0));
                    r.sx = sx;
                    r.sz = sz;
                }
                1 => {
                    x = i32::from(x < 0) - sz;
                    z = -(i32::from(z > 0));
                    r.sx = sz;
                    r.sz = sx;
                }
                2 => {
                    x = i32::from(x < 0) - sx;
                    z = i32::from(z < 0) - sz;
                    r.sx = sx;
                    r.sz = sz;
                }
                _ => {
                    x = -(i32::from(x > 0));
                    z = i32::from(z < 0) - sx;
                    r.sx = sz;
                    r.sz = sx;
                }
            }
            // city_anchor (13, *, 20) 是 city_center 的一部分
            let (sx, sz) = (13, 20); // city_anchor
            match r.rotation {
                0 => {
                    r.x = x - sx;
                    r.z = z - sz;
                }
                1 => {
                    r.x = x + sz;
                    r.z = z - sx;
                }
                2 => {
                    r.x = x + sx;
                    r.z = z + sz;
                }
                _ => {
                    r.x = x - sz;
                    r.z = z + sx;
                }
            }
            r.y = -27;
            r.sy = sy;
            Some(r)
        }

        RuinedPortal | RuinedPortalN => {
            // 废弃传送门分 7 类，各自在特定群系集合中独立生成；合起来每个群系
            // 恰好覆盖一次（deep_dark 除外）且无地形限制，因此每个 region 内
            // 总会生成一个。地下群系处可能判定失败或上下叠两个（群系检查在
            // 选定类型与高度后进行），该情形需要地表高度，cubiomes 也不支持。
            let cat = get_category(mc, biome_id);
            for &c in &[
                BiomeId::Desert as i32,
                BiomeId::Jungle as i32,
                BiomeId::Swamp as i32,
                BiomeId::Ocean as i32,
                BiomeId::NetherWastes as i32,
            ] {
                if cat == c {
                    r.biome = cat;
                }
            }
            if r.biome == -1 {
                use BiomeId::*;
                r.biome = match BiomeId::from_i32(biome_id) {
                    Some(MangroveSwamp) => Swamp as i32,
                    Some(
                        Mountains // windswept_hills
                        | MountainEdge
                        | WoodedMountains // windswept_forest
                        | GravellyMountains // windswept_gravelly_hills
                        | ModifiedGravellyMountains
                        | SavannaPlateau
                        | ShatteredSavanna // windswept_savanna
                        | ShatteredSavannaPlateau
                        | Badlands
                        | ErodedBadlands
                        | WoodedBadlandsPlateau // wooded_badlands
                        | ModifiedBadlandsPlateau
                        | ModifiedWoodedBadlandsPlateau
                        | SnowyTaigaMountains
                        | TaigaMountains
                        | StoneShore // stony_shore
                        | Meadow
                        | FrozenPeaks
                        | JaggedPeaks
                        | StonyPeaks
                        | SnowySlopes,
                    ) => Mountains as i32,
                    _ => -1,
                };
            }
            if r.biome == -1 {
                r.biome = BiomeId::Plains as i32;
            }
            if r.biome == BiomeId::Plains as i32 || r.biome == BiomeId::Mountains as i32 {
                r.underground = rng.next_float() < 0.5;
                if r.underground {
                    r.airpocket = true;
                } else {
                    r.airpocket = rng.next_float() < 0.5;
                }
            } else if r.biome == BiomeId::Jungle as i32 {
                r.airpocket = rng.next_float() < 0.5;
            }
            r.giant = rng.next_float() < 0.05;
            if r.giant {
                // ruined_portal/giant_portal_1..3
                r.start = 1 + rng.next_int_bound(3);
            } else {
                // ruined_portal/portal_1..10
                r.start = 1 + rng.next_int_bound(10);
            }
            r.rotation = rng.next_int_bound(4);
            r.mirror = rng.next_float() < 0.5;
            Some(r)
        }

        Monument => {
            r.x = -29;
            r.z = -29;
            r.sx = 58;
            r.sz = 58;
            Some(r)
        }

        Igloo => {
            if mc <= McVersion::V1_12 {
                rng = JavaRandom::new(
                    get_population_seed(mc, seed, (x >> 4) - 1, (z >> 4) - 1) as i64,
                );
            }
            r.rotation = rng.next_int_bound(4);
            r.basement = rng.next_double() < 0.5;
            r.size = rng.next_int_bound(8) + 4;
            let (sx, sy, sz) = (7, 5, 8);
            r.sy = sy;
            // 朝向: 0:north, 1:east, 2:south, 3:west
            match r.rotation {
                0 => {
                    r.rotation = 0;
                    r.mirror = false;
                    r.sx = sx;
                    r.sz = sz;
                }
                1 => {
                    r.rotation = 1;
                    r.mirror = false;
                    r.sx = sz;
                    r.sz = sx;
                }
                2 => {
                    r.rotation = 0;
                    r.mirror = true;
                    r.sx = sx;
                    r.sz = sz;
                }
                _ => {
                    r.rotation = 1;
                    r.mirror = true;
                    r.sx = sz;
                    r.sz = sx;
                }
            }
            Some(r)
        }

        DesertPyramid | JungleTemple | SwampHut => {
            let (sx, sy, sz) = match stype {
                DesertPyramid => (21, 15, 21),
                JungleTemple => (12, 10, 15),
                _ => (7, 7, 9), // SwampHut
            };
            r.sy = sy;
            if mc <= McVersion::V1_19 {
                r.sx = sx;
                r.sz = sz;
                return Some(r);
            }
            // 朝向: 0:north, 1:east, 2:south, 3:west
            match rng.next_int_bound(4) {
                0 => {
                    r.rotation = 0;
                    r.mirror = false;
                    r.sx = sx;
                    r.sz = sz;
                }
                1 => {
                    r.rotation = 1;
                    r.mirror = false;
                    r.sx = sz;
                    r.sz = sx;
                }
                2 => {
                    r.rotation = 0;
                    r.mirror = true;
                    r.sx = sx;
                    r.sz = sz;
                }
                _ => {
                    r.rotation = 1;
                    r.mirror = true;
                    r.sx = sz;
                    r.sz = sx;
                }
            }
            Some(r)
        }

        Geode => {
            let sc = get_config(Geode, mc)?;
            if mc >= McVersion::V1_18 {
                let mut xr = Xoroshiro::new(
                    get_population_seed(mc, seed, x & !15, z & !15)
                        .wrapping_add(sc.salt as i64 as u64),
                );
                if xr.next_float() >= sc.rarity {
                    return None;
                }
                r.x = xr.next_int_j(16); // 区块内偏移 X
                r.z = xr.next_int_j(16); // 区块内偏移 Z
                r.x -= x & 15; // 换算为相对 x/z 的偏移
                r.z -= z & 15;
                r.y = xr.next_int_j(1 + 30 + 58) - 58; // Y 高度
                r.size = xr.next_int_j(2) + 3; // 分布点数
                xr.skip(2);
                r.cracked = xr.next_float() < 0.95;
            } else {
                let mut rng = JavaRandom::new(
                    (get_population_seed(mc, seed, x & !15, z & !15)
                        .wrapping_add(sc.salt as i64 as u64)) as i64,
                );
                if rng.next_float() >= sc.rarity {
                    return None;
                }
                r.x = rng.next_int_bound(16);
                r.z = rng.next_int_bound(16);
                r.x -= x & 15;
                r.z -= z & 15;
                r.y = rng.next_int_bound(1 + 46 - 6) + 6;
                r.size = rng.next_int_bound(2) + 3;
                rng.skip(2);
                r.cracked = rng.next_float() < 0.95;
            }
            // 紫晶洞围绕一组点近似球形生成，各轴偏移 4-6
            r.x += 5;
            r.y += 5;
            r.z += 5;
            Some(r)
        }

        TrialChambers => {
            r.y = rng.next_int_bound(1 + 20) - 40; // Y 高度
            r.rotation = rng.next_int_bound(4);
            r.start = rng.next_int_bound(2); // corridor/end_[12]
            r.sx = 19;
            r.sy = 20;
            r.sz = 19;
            match r.rotation {
                0 => {}
                1 => {
                    r.x = 1 - r.sz;
                    r.z = 0;
                }
                2 => {
                    r.x = 1 - r.sx;
                    r.z = 1 - r.sz;
                }
                _ => {
                    r.x = 0;
                    r.z = 1 - r.sx;
                }
            }
            Some(r)
        }

        _ => None,
    }
}

/// C `getVariant` 的 `L_rotate_village_bastion`：村庄/堡垒的朝向与包围盒。
fn rotate_village_bastion(
    r: &mut StructureVariant,
    mc: McVersion,
    x: i32,
    z: i32,
    sx: i32,
    sy: i32,
    sz: i32,
) {
    r.sy = sy;
    if mc >= McVersion::V1_18 {
        // 0:0, 1:cw90, 2:cw180, 3:cw270=ccw90
        match r.rotation {
            0 => {
                r.x = 0;
                r.z = 0;
                r.sx = sx;
                r.sz = sz;
            }
            1 => {
                r.x = 1 - sz;
                r.z = 0;
                r.sx = sz;
                r.sz = sx;
            }
            2 => {
                r.x = 1 - sx;
                r.z = 1 - sz;
                r.sx = sx;
                r.sz = sz;
            }
            _ => {
                r.x = 0;
                r.z = 1 - sx;
                r.sx = sz;
                r.sz = sx;
            }
        }
    } else {
        // C 的 `(x<0)` 是 C 布尔转 int（0/1）
        match r.rotation {
            0 => {
                r.x = 0;
                r.z = 0;
                r.sx = sx;
                r.sz = sz;
            }
            1 => {
                r.x = i32::from(x < 0) - sz;
                r.z = 0;
                r.sx = sz;
                r.sz = sx;
            }
            2 => {
                r.x = i32::from(x < 0) - sx;
                r.z = i32::from(z < 0) - sz;
                r.sx = sx;
                r.sz = sz;
            }
            _ => {
                r.x = 0;
                r.z = i32::from(z < 0) - sx;
                r.sx = sz;
                r.sz = sx;
            }
        }
    }
}
