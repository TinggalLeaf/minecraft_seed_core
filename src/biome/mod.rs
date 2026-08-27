//! 生物群系 ID，与 cubiomes `biomes.h` 的 `BiomeID` 完全对齐（含 1.13 前
//! 的旧名称别名与 1.18 的重命名映射）。
//!
//! 数值 ID 即 Minecraft 注册表序号的 cubiomes 约定值，结构/群系查找
//! 依赖这些固定数值，请勿改动。

use crate::version::McVersion;

/// 生物群系 ID（`i32` 判别值与 cubiomes 一致）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(i32)]
pub enum BiomeId {
    None = -1,
    Ocean = 0,
    Plains = 1,
    Desert = 2,
    Mountains = 3, // extremeHills / windswept_hills
    Forest = 4,
    Taiga = 5,
    Swamp = 6,
    River = 7,
    NetherWastes = 8, // hell
    TheEnd = 9,       // sky
    FrozenOcean = 10,
    FrozenRiver = 11,
    SnowyTundra = 12, // icePlains / snowy_plains
    SnowyMountains = 13,
    MushroomFields = 14,
    MushroomFieldShore = 15,
    Beach = 16,
    DesertHills = 17,
    WoodedHills = 18,
    TaigaHills = 19,
    MountainEdge = 20,
    Jungle = 21,
    JungleHills = 22,
    JungleEdge = 23, // sparse_jungle
    DeepOcean = 24,
    StoneShore = 25, // stony_shore
    SnowyBeach = 26,
    BirchForest = 27,
    BirchForestHills = 28,
    DarkForest = 29, // roofedForest
    SnowyTaiga = 30,
    SnowyTaigaHills = 31,
    GiantTreeTaiga = 32, // megaTaiga / old_growth_pine_taiga
    GiantTreeTaigaHills = 33,
    WoodedMountains = 34, // extremeHillsPlus / windswept_forest
    Savanna = 35,
    SavannaPlateau = 36,
    Badlands = 37, // mesa
    WoodedBadlandsPlateau = 38, // mesaPlateau_F / wooded_badlands
    BadlandsPlateau = 39,
    // 1.13
    SmallEndIslands = 40,
    EndMidlands = 41,
    EndHighlands = 42,
    EndBarrens = 43,
    WarmOcean = 44,
    LukewarmOcean = 45,
    ColdOcean = 46,
    DeepWarmOcean = 47,
    DeepLukewarmOcean = 48,
    DeepColdOcean = 49,
    DeepFrozenOcean = 50,
    // Alpha 1.2 - Beta 1.7
    SeasonalForest = 51,
    Rainforest = 52,
    Shrubland = 53,

    TheVoid = 127,

    // 突变变体（base + 128）
    SunflowerPlains = 129,
    DesertLakes = 130,
    GravellyMountains = 131, // windswept_gravelly_hills
    FlowerForest = 132,
    TaigaMountains = 133,
    SwampHills = 134,
    IceSpikes = 140,
    ModifiedJungle = 149,
    ModifiedJungleEdge = 151,
    TallBirchForest = 155, // old_growth_birch_forest
    TallBirchHills = 156,
    DarkForestHills = 157,
    SnowyTaigaMountains = 158,
    GiantSpruceTaiga = 160, // old_growth_spruce_taiga
    GiantSpruceTaigaHills = 161,
    ModifiedGravellyMountains = 162,
    ShatteredSavanna = 163, // windswept_savanna
    ShatteredSavannaPlateau = 164,
    ErodedBadlands = 165,
    ModifiedWoodedBadlandsPlateau = 166,
    ModifiedBadlandsPlateau = 167,
    // 1.14
    BambooJungle = 168,
    BambooJungleHills = 169,
    // 1.16
    SoulSandValley = 170,
    CrimsonForest = 171,
    WarpedForest = 172,
    BasaltDeltas = 173,
    // 1.17
    DripstoneCaves = 174,
    LushCaves = 175,
    // 1.18
    Meadow = 177,
    Grove = 178,
    SnowySlopes = 179,
    JaggedPeaks = 180,
    FrozenPeaks = 181,
    StonyPeaks = 182,
    // 1.19
    DeepDark = 183,
    MangroveSwamp = 184,
    // 1.20
    CherryGrove = 185,
    // 1.21.4 (Winter Drop)
    PaleGarden = 186,
}

impl BiomeId {
    /// 从 i32 转换，未知值返回 `None`。
    pub fn from_i32(v: i32) -> Option<BiomeId> {
        if v == -1 {
            return Some(BiomeId::None);
        }
        // 判别值不连续（54..126、176 缺失等），用匹配保证与 cubiomes 一致。
        use BiomeId::*;
        Some(match v {
            0 => Ocean, 1 => Plains, 2 => Desert, 3 => Mountains, 4 => Forest,
            5 => Taiga, 6 => Swamp, 7 => River, 8 => NetherWastes, 9 => TheEnd,
            10 => FrozenOcean, 11 => FrozenRiver, 12 => SnowyTundra,
            13 => SnowyMountains, 14 => MushroomFields, 15 => MushroomFieldShore,
            16 => Beach, 17 => DesertHills, 18 => WoodedHills, 19 => TaigaHills,
            20 => MountainEdge, 21 => Jungle, 22 => JungleHills, 23 => JungleEdge,
            24 => DeepOcean, 25 => StoneShore, 26 => SnowyBeach, 27 => BirchForest,
            28 => BirchForestHills, 29 => DarkForest, 30 => SnowyTaiga,
            31 => SnowyTaigaHills, 32 => GiantTreeTaiga, 33 => GiantTreeTaigaHills,
            34 => WoodedMountains, 35 => Savanna, 36 => SavannaPlateau,
            37 => Badlands, 38 => WoodedBadlandsPlateau, 39 => BadlandsPlateau,
            40 => SmallEndIslands, 41 => EndMidlands, 42 => EndHighlands,
            43 => EndBarrens, 44 => WarmOcean, 45 => LukewarmOcean,
            46 => ColdOcean, 47 => DeepWarmOcean, 48 => DeepLukewarmOcean,
            49 => DeepColdOcean, 50 => DeepFrozenOcean,
            51 => SeasonalForest, 52 => Rainforest, 53 => Shrubland,
            127 => TheVoid,
            129 => SunflowerPlains, 130 => DesertLakes, 131 => GravellyMountains,
            132 => FlowerForest, 133 => TaigaMountains, 134 => SwampHills,
            140 => IceSpikes, 149 => ModifiedJungle, 151 => ModifiedJungleEdge,
            155 => TallBirchForest, 156 => TallBirchHills, 157 => DarkForestHills,
            158 => SnowyTaigaMountains, 160 => GiantSpruceTaiga,
            161 => GiantSpruceTaigaHills, 162 => ModifiedGravellyMountains,
            163 => ShatteredSavanna, 164 => ShatteredSavannaPlateau,
            165 => ErodedBadlands, 166 => ModifiedWoodedBadlandsPlateau,
            167 => ModifiedBadlandsPlateau, 168 => BambooJungle,
            169 => BambooJungleHills, 170 => SoulSandValley, 171 => CrimsonForest,
            172 => WarpedForest, 173 => BasaltDeltas, 174 => DripstoneCaves,
            175 => LushCaves, 177 => Meadow, 178 => Grove, 179 => SnowySlopes,
            180 => JaggedPeaks, 181 => FrozenPeaks, 182 => StonyPeaks,
            183 => DeepDark, 184 => MangroveSwamp, 185 => CherryGrove,
            186 => PaleGarden,
            _ => return Option::None,
        })
    }

    /// 该群系在指定版本中是否存在（对应 cubiomes `biomeExists`）。
    pub fn exists_in(self, mc: McVersion) -> bool {
        use BiomeId::*;
        let id = self as i32;
        // 各版本新增群系的引入版本
        let introduced = match self {
            SmallEndIslands | EndMidlands | EndHighlands | EndBarrens
            | WarmOcean | LukewarmOcean | ColdOcean | DeepWarmOcean
            | DeepLukewarmOcean | DeepColdOcean | DeepFrozenOcean | TheVoid => McVersion::V1_13,
            BambooJungle | BambooJungleHills => McVersion::V1_14,
            SoulSandValley | CrimsonForest | WarpedForest | BasaltDeltas => McVersion::V1_16,
            DripstoneCaves | LushCaves => McVersion::V1_17,
            Meadow | Grove | SnowySlopes | JaggedPeaks | FrozenPeaks | StonyPeaks => {
                McVersion::V1_18
            }
            DeepDark | MangroveSwamp => McVersion::V1_19,
            CherryGrove => McVersion::V1_20,
            PaleGarden => McVersion::V1_21,
            _ => McVersion::V1_7,
        };
        if mc < introduced {
            return false;
        }
        // 1.18 起移除的变体群系（山丘、边缘、浅滩等）
        if mc >= McVersion::V1_18 {
            let removed = matches!(
                self,
                DesertHills | WoodedHills | TaigaHills | MountainEdge | JungleHills
                    | SnowyTaigaHills | GiantTreeTaigaHills | SnowyMountains
                    | MushroomFieldShore | SwampHills | TaigaMountains | DarkForestHills
                    | SnowyTaigaMountains | GiantSpruceTaigaHills | ModifiedGravellyMountains
                    | ShatteredSavannaPlateau | ModifiedWoodedBadlandsPlateau
                    | ModifiedBadlandsPlateau | BambooJungleHills | TallBirchHills
                    | GravellyMountains | IceSpikes | ModifiedJungle | ModifiedJungleEdge
                    | DesertLakes | SunflowerPlains | FlowerForest | DeepWarmOcean
            );
            if removed {
                return false;
            }
        }
        let _ = id;
        true
    }
}

// ============================================================================
// 分层群系生成（1.7–1.17）使用的分类/相似性助手，对应 cubiomes `biomes.c`。
// 均作用于原始 i32 群系 ID（分层阶段还会出现非群系的温度分类中间值，
// 故不挂在 `BiomeId` 上）。
// ============================================================================

/// `getCategory`：返回群系的分类代表 ID（如所有丛林变体归为 `jungle`）。
///
/// 未知 ID 返回 `none`（-1）。`wooded_badlands_plateau`/`badlands_plateau`
/// 在 1.15 及之前归为 `mesa`（= `badlands`），1.16+ 起独立为
/// `badlands_plateau`（对应 C 的 `mc <= MC_1_15` 分支）。
pub fn get_category(mc: McVersion, id: i32) -> i32 {
    use BiomeId::*;
    let b = match BiomeId::from_i32(id) {
        Some(b) => b,
        Option::None => return None as i32,
    };
    let cat = match b {
        Beach | SnowyBeach => Beach,
        Desert | DesertHills | DesertLakes => Desert,
        Mountains | MountainEdge | WoodedMountains | GravellyMountains
        | ModifiedGravellyMountains => Mountains,
        Forest | WoodedHills | BirchForest | BirchForestHills | DarkForest | FlowerForest
        | TallBirchForest | TallBirchHills | DarkForestHills => Forest,
        SnowyTundra | SnowyMountains | IceSpikes => SnowyTundra,
        Jungle | JungleHills | JungleEdge | ModifiedJungle | ModifiedJungleEdge
        | BambooJungle | BambooJungleHills => Jungle,
        Badlands | ErodedBadlands | ModifiedWoodedBadlandsPlateau
        | ModifiedBadlandsPlateau => Badlands, // mesa = badlands
        WoodedBadlandsPlateau | BadlandsPlateau => {
            if mc <= McVersion::V1_15 {
                Badlands // mesa
            } else {
                BadlandsPlateau
            }
        }
        MushroomFields | MushroomFieldShore => MushroomFields,
        StoneShore => StoneShore,
        Ocean | FrozenOcean | DeepOcean | WarmOcean | LukewarmOcean | ColdOcean
        | DeepWarmOcean | DeepLukewarmOcean | DeepColdOcean | DeepFrozenOcean => Ocean,
        Plains | SunflowerPlains => Plains,
        River | FrozenRiver => River,
        Savanna | SavannaPlateau | ShatteredSavanna | ShatteredSavannaPlateau => Savanna,
        Swamp | SwampHills => Swamp,
        Taiga | TaigaHills | SnowyTaiga | SnowyTaigaHills | GiantTreeTaiga
        | GiantTreeTaigaHills | TaigaMountains | SnowyTaigaMountains | GiantSpruceTaiga
        | GiantSpruceTaigaHills => Taiga,
        NetherWastes | SoulSandValley | CrimsonForest | WarpedForest | BasaltDeltas => {
            NetherWastes
        }
        _ => return None as i32,
    };
    cat as i32
}

/// `areSimilar`：两群系是否"相似"（同分类；1.15- 的恶地高原互相相似）。
pub fn are_similar(mc: McVersion, id1: i32, id2: i32) -> bool {
    if id1 == id2 {
        return true;
    }
    if mc <= McVersion::V1_15 {
        let p = [BiomeId::WoodedBadlandsPlateau as i32, BiomeId::BadlandsPlateau as i32];
        if p.contains(&id1) {
            return p.contains(&id2);
        }
    }
    get_category(mc, id1) == get_category(mc, id2)
}

/// `getMutated`：群系的突变（"M" 变体）ID，无突变返回 `none`（-1）。
///
/// 含 MC-98995 的模拟：1.9–1.10 中 `birch_forest` 错误地突变为
/// `tall_birch_hills`，而 `birch_forest_hills` 无突变。
pub fn get_mutated(mc: McVersion, id: i32) -> i32 {
    use BiomeId::*;
    let b = match BiomeId::from_i32(id) {
        Some(b) => b,
        Option::None => return None as i32,
    };
    let m = match b {
        Plains => SunflowerPlains,
        Desert => DesertLakes,
        Mountains => GravellyMountains,
        Forest => FlowerForest,
        Taiga => TaigaMountains,
        Swamp => SwampHills,
        SnowyTundra => IceSpikes,
        Jungle => ModifiedJungle,
        JungleEdge => ModifiedJungleEdge,
        BirchForest => {
            if (McVersion::V1_9..=McVersion::V1_10).contains(&mc) {
                TallBirchHills // MC-98995
            } else {
                TallBirchForest
            }
        }
        BirchForestHills => {
            if (McVersion::V1_9..=McVersion::V1_10).contains(&mc) {
                return None as i32; // MC-98995
            }
            TallBirchHills
        }
        DarkForest => DarkForestHills,
        SnowyTaiga => SnowyTaigaMountains,
        GiantTreeTaiga => GiantSpruceTaiga,
        GiantTreeTaigaHills => GiantSpruceTaigaHills,
        WoodedMountains => ModifiedGravellyMountains,
        Savanna => ShatteredSavanna,
        SavannaPlateau => ShatteredSavannaPlateau,
        Badlands => ErodedBadlands,
        WoodedBadlandsPlateau => ModifiedWoodedBadlandsPlateau,
        BadlandsPlateau => ModifiedBadlandsPlateau,
        _ => return None as i32,
    };
    m as i32
}

/// `isMesa`：恶地（含高原与变体）。
pub fn is_mesa(id: i32) -> bool {
    use BiomeId::*;
    matches!(
        BiomeId::from_i32(id),
        Some(
            Badlands | ErodedBadlands | ModifiedWoodedBadlandsPlateau
            | ModifiedBadlandsPlateau | WoodedBadlandsPlateau | BadlandsPlateau
        )
    )
}

/// `isShallowOcean`：浅海（ocean/frozen/warm/lukewarm/cold）。
pub fn is_shallow_ocean(id: i32) -> bool {
    use BiomeId::*;
    matches!(
        BiomeId::from_i32(id),
        Some(Ocean | FrozenOcean | WarmOcean | LukewarmOcean | ColdOcean)
    )
}

/// `isDeepOcean`：深海（五种 deep_*_ocean）。
pub fn is_deep_ocean(id: i32) -> bool {
    use BiomeId::*;
    matches!(
        BiomeId::from_i32(id),
        Some(
            DeepOcean | DeepWarmOcean | DeepLukewarmOcean | DeepColdOcean | DeepFrozenOcean
        )
    )
}

/// `isOceanic`：任意海洋（浅海 + 深海）。
pub fn is_oceanic(id: i32) -> bool {
    is_shallow_ocean(id) || is_deep_ocean(id)
}

/// `isSnowy`：冰雪类群系。
pub fn is_snowy(id: i32) -> bool {
    use BiomeId::*;
    matches!(
        BiomeId::from_i32(id),
        Some(
            FrozenOcean | FrozenRiver | SnowyTundra | SnowyMountains | SnowyBeach
            | SnowyTaiga | SnowyTaigaHills | IceSpikes | SnowyTaigaMountains
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_roundtrip() {
        for v in [
            -1, 0, 1, 8, 37, 50, 53, 127, 129, 168, 170, 175, 177, 183, 185, 186,
        ] {
            let b = BiomeId::from_i32(v).unwrap();
            assert_eq!(b as i32, v);
        }
        assert!(BiomeId::from_i32(54).is_none());
        assert!(BiomeId::from_i32(176).is_none());
    }

    #[test]
    fn version_gating() {
        assert!(!BiomeId::CherryGrove.exists_in(McVersion::V1_19));
        assert!(BiomeId::CherryGrove.exists_in(McVersion::V1_20));
        assert!(!BiomeId::PaleGarden.exists_in(McVersion::V1_21_3));
        assert!(BiomeId::PaleGarden.exists_in(McVersion::V1_21));
        assert!(BiomeId::DesertHills.exists_in(McVersion::V1_17));
        assert!(!BiomeId::DesertHills.exists_in(McVersion::V1_18));
    }

    #[test]
    fn category_and_similarity() {
        use BiomeId::*;
        // 恶地高原在 1.15- 归为 mesa，1.16+ 起独立
        assert_eq!(
            get_category(McVersion::V1_15, WoodedBadlandsPlateau as i32),
            Badlands as i32
        );
        assert_eq!(
            get_category(McVersion::V1_16, WoodedBadlandsPlateau as i32),
            BadlandsPlateau as i32
        );
        assert!(are_similar(
            McVersion::V1_14,
            WoodedBadlandsPlateau as i32,
            BadlandsPlateau as i32
        ));
        assert!(are_similar(McVersion::V1_12, Jungle as i32, JungleHills as i32));
        assert!(!are_similar(McVersion::V1_12, Jungle as i32, Desert as i32));
        assert_eq!(get_category(McVersion::V1_12, 999), None as i32);
    }

    #[test]
    fn mutated_variants() {
        use BiomeId::*;
        assert_eq!(get_mutated(McVersion::V1_12, Plains as i32), SunflowerPlains as i32);
        // MC-98995：仅 1.9–1.10
        assert_eq!(get_mutated(McVersion::V1_9, BirchForest as i32), TallBirchHills as i32);
        assert_eq!(get_mutated(McVersion::V1_10, BirchForestHills as i32), None as i32);
        assert_eq!(get_mutated(McVersion::V1_11, BirchForest as i32), TallBirchForest as i32);
        assert_eq!(get_mutated(McVersion::V1_12, Ocean as i32), None as i32);
    }

    #[test]
    fn ocean_predicates() {
        use BiomeId::*;
        assert!(is_shallow_ocean(Ocean as i32));
        assert!(is_shallow_ocean(FrozenOcean as i32));
        assert!(!is_shallow_ocean(DeepOcean as i32));
        assert!(is_oceanic(DeepWarmOcean as i32));
        assert!(!is_oceanic(River as i32));
        // 注意：分层中间值 Warm=1（数值等于 plains）不是海洋
        assert!(!is_shallow_ocean(Plains as i32));
        assert!(is_snowy(IceSpikes as i32));
        assert!(!is_snowy(Taiga as i32));
        assert!(is_mesa(ErodedBadlands as i32));
        assert!(!is_mesa(Desert as i32));
    }
}
