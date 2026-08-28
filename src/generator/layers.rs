//! B1.8–1.17 主世界分层（LayerStack）群系生成，移植自 cubiomes
//! `layers.c` / `layers.h` 与 `generator.c` 的 `setupLayerStack`。
//!
//! ## 结构对应
//!
//! - [`LayerStack`] ↔ C `LayerStack`：固定槽位的层数组 + 各比例入口层。
//! - [`Layer`] ↔ C `Layer`：`map` 函数指针、`mc`、`layer_salt/start_salt/
//!   start_seed` 与两个父层索引。C 的 `zoom/edge/scale` 字段仅用于
//!   `getMaxArea` 的缓冲区预估；本实现每层按需分配精确缓冲，故省略。
//! - 层函数 ↔ C 同名 `mapfunc_t`（[`map_continent`] 等），逐函数移植，
//!   salt 数值与 `setupLayerStack` 完全一致。
//!
//! ## 与 C 的行为差异（仅实现层面，输出逐位一致）
//!
//! - C 在单个预分配缓冲上以 `out + pW*pH` 复用暂存区；这里每层用独立
//!   `Vec`，避免一切别名/越界问题。
//! - C 的 `mapZoom`/`mapZoomFuzzy` 循环会多读父缓冲末尾之后的一行又 1 格
//!   （越界读，读到的是预留缓冲区里的垃圾，但这些值只影响永远不会被拷贝
//!   到输出的边缘格）。这里父缓冲多分配一行零一格并清零，输出与 C 一致。
//! - 注意 `mapLand` 中 C 的 `case forest:` / `v != forest` 里的 `forest`（4）
//!   在 1.7+ 链中实际匹配的是温度分类 `Freezing`（同为 4）——群系 ID 与
//!   温度分类在中间层共享数值空间，移植时保留该语义。
//! - `mapVoronoi114` 的核心算法抽出为
//!   [`crate::generator::voronoi::map_voronoi_114_plane`]（末地 1.9–1.14
//!   的 scale 1 路径共用）。C 把结果写进 `out` 之后的暂存区再 `memmove`
//!   回来；循环覆盖全部输出格，直接写 `out` 等价。
//! - `mapOceanMixMod`（generator.c 的 `FORCE_OCEAN_VARIANTS` 路径）移植为
//!   纯混合函数 [`map_ocean_mix_mod`] + 区域入口
//!   [`LayerStack::gen_area_ocean_mix_mod`]（scale 16/64/256）。
//! - `mapOceanTemp` 的 Perlin 采样把世界 `(x, z)` 映射到噪声的 `(d1, d2)`
//!   轴（即噪声 "y" 轴是世界 z），按 C 原样保留。
//! - C 的 `mapVoronoi`（1.15+）先把父层数据写进 `out` 再移到暂存区，
//!   `mapVoronoiPlane` 覆盖不到的边缘输出格（逐点查询且 `(x-4) mod 4 ∈
//!   {2,3}` 时会出现）残留的是父层数据而非 0；本实现逐位复刻该行为。

use crate::biome::{
    are_similar, get_category, get_mutated, is_deep_ocean, is_mesa, is_oceanic,
    is_shallow_ocean, is_snowy, BiomeId,
};
use crate::noise::perlin::PerlinNoise;
use crate::rng::java::JavaRandom;
use crate::rng::seed::{chunk_seed, first_int, first_is_zero, layer_salt, start_salt, step_seed};
use crate::version::McVersion;
use crate::version::McVersion::{V1_0, V1_6, V1_7};

use super::voronoi::{get_voronoi_sha, map_voronoi_plane};
use super::Range;

#[cfg(test)]
mod tests;

/// `LAYER_INIT_SHA`：1.15+ voronoi 层用 SHA-256 初始化的标记盐。
const LAYER_INIT_SHA: u64 = !0u64;

// ============================================================================
// 温度分类中间值（`enum BiomeTempCategory`）与常用群系 ID 常量
// ============================================================================

/// `Oceanic` = 0（数值与 `ocean` 相同，按上下文区分含义）。
const CAT_OCEANIC: i32 = 0;
/// `Warm` = 1。
const CAT_WARM: i32 = 1;
/// `Lush` = 2。
const CAT_LUSH: i32 = 2;
/// `Cold` = 3。
const CAT_COLD: i32 = 3;
/// `Freezing` = 4。
const CAT_FREEZING: i32 = 4;

const OCEAN: i32 = BiomeId::Ocean as i32;
const PLAINS: i32 = BiomeId::Plains as i32;
const DESERT: i32 = BiomeId::Desert as i32;
const MOUNTAINS: i32 = BiomeId::Mountains as i32;
const FOREST: i32 = BiomeId::Forest as i32;
const TAIGA: i32 = BiomeId::Taiga as i32;
const SWAMP: i32 = BiomeId::Swamp as i32;
const RIVER: i32 = BiomeId::River as i32;
const FROZEN_OCEAN: i32 = BiomeId::FrozenOcean as i32;
const FROZEN_RIVER: i32 = BiomeId::FrozenRiver as i32;
const SNOWY_TUNDRA: i32 = BiomeId::SnowyTundra as i32;
const SNOWY_MOUNTAINS: i32 = BiomeId::SnowyMountains as i32;
const MUSHROOM_FIELDS: i32 = BiomeId::MushroomFields as i32;
const MUSHROOM_FIELD_SHORE: i32 = BiomeId::MushroomFieldShore as i32;
const BEACH: i32 = BiomeId::Beach as i32;
const DESERT_HILLS: i32 = BiomeId::DesertHills as i32;
const WOODED_HILLS: i32 = BiomeId::WoodedHills as i32;
const TAIGA_HILLS: i32 = BiomeId::TaigaHills as i32;
const MOUNTAIN_EDGE: i32 = BiomeId::MountainEdge as i32;
const JUNGLE: i32 = BiomeId::Jungle as i32;
const JUNGLE_HILLS: i32 = BiomeId::JungleHills as i32;
const JUNGLE_EDGE: i32 = BiomeId::JungleEdge as i32;
const DEEP_OCEAN: i32 = BiomeId::DeepOcean as i32;
const STONE_SHORE: i32 = BiomeId::StoneShore as i32;
const SNOWY_BEACH: i32 = BiomeId::SnowyBeach as i32;
const BIRCH_FOREST: i32 = BiomeId::BirchForest as i32;
const BIRCH_FOREST_HILLS: i32 = BiomeId::BirchForestHills as i32;
const DARK_FOREST: i32 = BiomeId::DarkForest as i32;
const SNOWY_TAIGA: i32 = BiomeId::SnowyTaiga as i32;
const SNOWY_TAIGA_HILLS: i32 = BiomeId::SnowyTaigaHills as i32;
const GIANT_TREE_TAIGA: i32 = BiomeId::GiantTreeTaiga as i32;
const GIANT_TREE_TAIGA_HILLS: i32 = BiomeId::GiantTreeTaigaHills as i32;
const WOODED_MOUNTAINS: i32 = BiomeId::WoodedMountains as i32;
const SAVANNA: i32 = BiomeId::Savanna as i32;
const SAVANNA_PLATEAU: i32 = BiomeId::SavannaPlateau as i32;
const BADLANDS: i32 = BiomeId::Badlands as i32;
const WOODED_BADLANDS_PLATEAU: i32 = BiomeId::WoodedBadlandsPlateau as i32;
const BADLANDS_PLATEAU: i32 = BiomeId::BadlandsPlateau as i32;
const WARM_OCEAN: i32 = BiomeId::WarmOcean as i32;
const LUKEWARM_OCEAN: i32 = BiomeId::LukewarmOcean as i32;
const COLD_OCEAN: i32 = BiomeId::ColdOcean as i32;
const DEEP_WARM_OCEAN: i32 = BiomeId::DeepWarmOcean as i32;
const DEEP_LUKEWARM_OCEAN: i32 = BiomeId::DeepLukewarmOcean as i32;
const DEEP_COLD_OCEAN: i32 = BiomeId::DeepColdOcean as i32;
const DEEP_FROZEN_OCEAN: i32 = BiomeId::DeepFrozenOcean as i32;
const SUNFLOWER_PLAINS: i32 = BiomeId::SunflowerPlains as i32;
const BAMBOO_JUNGLE: i32 = BiomeId::BambooJungle as i32;
const BAMBOO_JUNGLE_HILLS: i32 = BiomeId::BambooJungleHills as i32;

// ============================================================================
// 层槽位（`enum LayerId`；仅 1.7+ 栈用到的槽位有实际含义）
// ============================================================================

/// 层索引，数值与 cubiomes `enum LayerId` 一致（便于与 C 对照调试）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
#[allow(dead_code)] // 部分槽位仅 1.6-/beta 栈使用，为对齐 C 序号保留
pub(crate) enum LayerId {
    Continent4096 = 0, // L_CONTINENT_4096
    Zoom4096,          // L_ZOOM_4096（beta）
    Land4096,          // L_LAND_4096（beta）
    Zoom2048,
    Land2048,
    Zoom1024,
    Land1024A,
    Land1024B,
    Land1024C,
    Island1024,
    Snow1024,
    Land1024D,
    Cool1024,
    Heat1024,
    Special1024,
    Zoom512,
    Land512,
    Zoom256,
    Land256,
    Mushroom256,
    DeepOcean256,
    Biome256,
    Bamboo256,
    Zoom128,
    Zoom64,
    BiomeEdge64,
    RiverInit256, // L_NOISE_256 / L_RIVER_INIT_256
    Zoom128Hills,
    Zoom64Hills,
    Hills64,
    Sunflower64,
    Zoom32,
    Land32,
    Zoom16,
    Shore16,
    SwampRiver16,
    Zoom8,
    Zoom4,
    Smooth4,
    Zoom128River,
    Zoom64River,
    Zoom32River,
    Zoom16River,
    Zoom8River,
    Zoom4River,
    River4,
    Smooth4River,
    RiverMix4,
    OceanTemp256,
    Zoom128Ocean,
    Zoom64Ocean,
    Zoom32Ocean,
    Zoom16Ocean,
    Zoom8Ocean,
    Zoom4Ocean,
    OceanMix4,
    Voronoi1,
    ZoomLargeA,
    ZoomLargeB,
    ZoomLRiverA,
    ZoomLRiverB,
}

/// `L_NUM`。
const L_NUM: usize = LayerId::ZoomLRiverB as usize + 1;

// ============================================================================
// Layer / LayerStack
// ============================================================================

/// 层函数签名（对应 C `mapfunc_t`）：`(层栈, 本层索引, 输出, x, z, w, h)`。
type MapFn = fn(&LayerStack, usize, &mut [i32], i32, i32, i32, i32);

/// 单层（对应 C `Layer`；`zoom/edge/scale/noise/data` 省略，见模块文档）。
#[derive(Clone, Copy, Debug)]
struct Layer {
    map: MapFn,
    mc: McVersion,
    /// 处理后的盐（`setupLayer` 中 `saltbase` 经 `getLayerSalt` 处理；
    /// 0 与 [`LAYER_INIT_SHA`] 为特殊值）。
    layer_salt: u64,
    start_salt: u64,
    start_seed: u64,
    p: Option<usize>,
    p2: Option<usize>,
}

impl Default for Layer {
    fn default() -> Self {
        // 未被当前版本栈使用的槽位；不会被调用
        Layer {
            map: map_continent,
            mc: McVersion::V1_7,
            layer_salt: 0,
            start_salt: 0,
            start_seed: 0,
            p: None,
            p2: None,
        }
    }
}

/// 层栈（对应 C `LayerStack`），B1.8–1.17 主世界群系生成。
#[derive(Clone, Debug)]
pub struct LayerStack {
    layers: Vec<Layer>,
    entry_1: usize,
    entry_4: usize,
    entry_16: usize,
    entry_64: usize,
    entry_256: usize,
    /// 1.13+ 海洋温度噪声（`LayerStack.oceanRnd`）。
    ocean_rnd: PerlinNoise,
    /// 结构可行性过滤器（对应 C `isViableStructurePos` 临时把
    /// `L_BIOME_256`/`L_SHORE_16` 的 getMap 换成 `mapViableBiome`/
    /// `mapViableShore`）：设置后，这两层生成的区域若不含目标群系，
    /// 本次查询整体判失败（C 的 err 传播 → `getBiomeAt` 返回 `none`）。
    pub(crate) viable_filter: std::cell::Cell<Option<(crate::structure::StructureType, McVersion)>>,
    /// 当前查询是否触发了可行性剪枝（每次查询前清零、查询后读取）。
    pub(crate) viable_failed: std::cell::Cell<bool>,
}

impl LayerStack {
    /// `setupLayerStack`：按版本组装 B1.8–1.17 的层链。
    ///
    /// `large_biomes` 对应 C 的 `LARGE_BIOMES` 标志（额外两级 zoom，
    /// 1.7 的河流链也额外两级）；1.3 之前该标志被忽略（同 C）。
    pub fn new(mc: McVersion, large_biomes: bool) -> Self {
        debug_assert!((McVersion::B1_8..=McVersion::V1_17).contains(&mc));
        let large_biomes = large_biomes && mc >= McVersion::V1_3;
        let mut layers = vec![Layer::default(); L_NUM];

        // `setupLayer`：返回层索引以便链式引用
        let setup =
            |layers: &mut Vec<Layer>, id: LayerId, map: MapFn, saltbase: u64,
             p: Option<usize>, p2: Option<usize>| {
                let layer_salt = if saltbase == 0 || saltbase == LAYER_INIT_SHA {
                    saltbase
                } else {
                    layer_salt(saltbase)
                };
                layers[id as usize] = Layer {
                    map,
                    mc,
                    layer_salt,
                    start_salt: 0,
                    start_seed: 0,
                    p,
                    p2,
                };
                id as usize
            };

        use LayerId as L;
        // 旧版栈的陆地扩张函数（C 的 `map_land` 变量）
        let map_land_fn: MapFn = if mc == McVersion::B1_8 {
            map_land_b18
        } else if mc <= McVersion::V1_6 {
            map_land16
        } else {
            map_land
        };

        let mut p;
        if mc == McVersion::B1_8 {
            // ---- Beta 1.8 主链（注意 Continent4096 槽位实际是 1:8192）----
            p = setup(&mut layers, L::Continent4096, map_continent, 1, None, None);
            p = setup(&mut layers, L::Zoom4096, map_zoom_fuzzy, 2000, Some(p), None);
            p = setup(&mut layers, L::Land4096, map_land_fn, 1, Some(p), None);
            p = setup(&mut layers, L::Zoom2048, map_zoom, 2001, Some(p), None);
            p = setup(&mut layers, L::Land2048, map_land_fn, 2, Some(p), None);
            p = setup(&mut layers, L::Zoom1024, map_zoom, 2002, Some(p), None);
            p = setup(&mut layers, L::Land1024A, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Zoom512, map_zoom, 2003, Some(p), None);
            p = setup(&mut layers, L::Land512, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Zoom256, map_zoom, 2004, Some(p), None);
            p = setup(&mut layers, L::Land256, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Biome256, map_biome, 200, Some(p), None);
            p = setup(&mut layers, L::Zoom128, map_zoom, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom64, map_zoom, 1001, Some(p), None);
            // 河流噪声链，同时驱动 hills 的判定
            setup(
                &mut layers,
                L::RiverInit256,
                map_noise,
                100,
                Some(L::Land256 as usize),
                None,
            );
        } else if mc <= McVersion::V1_6 {
            // ---- 1.0–1.6 主链 ----
            p = setup(&mut layers, L::Continent4096, map_continent, 1, None, None);
            p = setup(&mut layers, L::Zoom2048, map_zoom_fuzzy, 2000, Some(p), None);
            p = setup(&mut layers, L::Land2048, map_land_fn, 1, Some(p), None);
            p = setup(&mut layers, L::Zoom1024, map_zoom, 2001, Some(p), None);
            p = setup(&mut layers, L::Land1024A, map_land_fn, 2, Some(p), None);
            p = setup(&mut layers, L::Snow1024, map_snow16, 2, Some(p), None);
            p = setup(&mut layers, L::Zoom512, map_zoom, 2002, Some(p), None);
            p = setup(&mut layers, L::Land512, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Zoom256, map_zoom, 2003, Some(p), None);
            p = setup(&mut layers, L::Land256, map_land_fn, 4, Some(p), None);
            p = setup(&mut layers, L::Mushroom256, map_mushroom, 5, Some(p), None);
            p = setup(&mut layers, L::Biome256, map_biome, 200, Some(p), None);
            p = setup(&mut layers, L::Zoom128, map_zoom, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom64, map_zoom, 1001, Some(p), None);
            // 河流噪声链，同时驱动 hills 的判定
            setup(
                &mut layers,
                L::RiverInit256,
                map_noise,
                100,
                Some(L::Mushroom256 as usize),
                None,
            );
        } else {
            // ---- 1.7+ 主链（`setupLayerStack` 的 `else` 分支）----
            p = setup(&mut layers, L::Continent4096, map_continent, 1, None, None);
            p = setup(&mut layers, L::Zoom2048, map_zoom_fuzzy, 2000, Some(p), None);
            p = setup(&mut layers, L::Land2048, map_land_fn, 1, Some(p), None);
            p = setup(&mut layers, L::Zoom1024, map_zoom, 2001, Some(p), None);
            p = setup(&mut layers, L::Land1024A, map_land_fn, 2, Some(p), None);
            p = setup(&mut layers, L::Land1024B, map_land_fn, 50, Some(p), None);
            p = setup(&mut layers, L::Land1024C, map_land_fn, 70, Some(p), None);
            p = setup(&mut layers, L::Island1024, map_island, 2, Some(p), None);
            p = setup(&mut layers, L::Snow1024, map_snow, 2, Some(p), None);
            p = setup(&mut layers, L::Land1024D, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Cool1024, map_cool, 2, Some(p), None);
            p = setup(&mut layers, L::Heat1024, map_heat, 2, Some(p), None);
            p = setup(&mut layers, L::Special1024, map_special, 3, Some(p), None);
            p = setup(&mut layers, L::Zoom512, map_zoom, 2002, Some(p), None);
            p = setup(&mut layers, L::Zoom256, map_zoom, 2003, Some(p), None);
            p = setup(&mut layers, L::Land256, map_land_fn, 4, Some(p), None);
            p = setup(&mut layers, L::Mushroom256, map_mushroom, 5, Some(p), None);
            p = setup(&mut layers, L::DeepOcean256, map_deep_ocean, 4, Some(p), None);
            p = setup(&mut layers, L::Biome256, map_biome, 200, Some(p), None);
            if mc >= McVersion::V1_14 {
                p = setup(&mut layers, L::Bamboo256, map_bamboo, 1001, Some(p), None);
            }
            p = setup(&mut layers, L::Zoom128, map_zoom, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom64, map_zoom, 1001, Some(p), None);
            setup(&mut layers, L::BiomeEdge64, map_biome_edge, 1000, Some(p), None);
            // 河流噪声链，同时驱动 hills 的判定
            p = setup(
                &mut layers,
                L::RiverInit256,
                map_noise,
                100,
                Some(L::DeepOcean256 as usize),
                None,
            );
        }

        // hills 分支的两级 zoom：1.0- 不存在；1.12- 盐为 0（startSalt/
        // startSeed 保持 0，对应 C 注释 "Pre 1.13 the Hills branch stays
        // zero-initialized"）
        if mc <= McVersion::V1_0 {
            // p 保持 Zoom64（不参与 hills）
        } else if mc <= McVersion::V1_12 {
            p = setup(&mut layers, L::Zoom128Hills, map_zoom, 0, Some(L::RiverInit256 as usize), None);
            setup(&mut layers, L::Zoom64Hills, map_zoom, 0, Some(p), None);
        } else {
            p = setup(&mut layers, L::Zoom128Hills, map_zoom, 1000, Some(p), None);
            setup(&mut layers, L::Zoom64Hills, map_zoom, 1001, Some(p), None);
        }

        if mc <= McVersion::V1_0 {
            // ---- B1.8/1.0 尾链（无 hills、无 SwampRiver；
            // 注意 Shore16 槽位实际是 1:32）----
            p = setup(&mut layers, L::Zoom32, map_zoom, 1000, Some(L::Zoom64 as usize), None);
            p = setup(&mut layers, L::Land32, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Shore16, map_shore, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom16, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Zoom8, map_zoom, 1002, Some(p), None);
            p = setup(&mut layers, L::Zoom4, map_zoom, 1003, Some(p), None);
            setup(&mut layers, L::Smooth4, map_smooth, 1000, Some(p), None);

            // 河流链
            p = setup(
                &mut layers,
                L::Zoom128River,
                map_zoom,
                1000,
                Some(L::RiverInit256 as usize),
                None,
            );
            p = setup(&mut layers, L::Zoom64River, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Zoom32River, map_zoom, 1002, Some(p), None);
            p = setup(&mut layers, L::Zoom16River, map_zoom, 1003, Some(p), None);
            p = setup(&mut layers, L::Zoom8River, map_zoom, 1004, Some(p), None);
            p = setup(&mut layers, L::Zoom4River, map_zoom, 1005, Some(p), None);
            p = setup(&mut layers, L::River4, map_river, 1, Some(p), None);
            setup(&mut layers, L::Smooth4River, map_smooth, 1000, Some(p), None);
        } else if mc <= McVersion::V1_6 {
            // ---- 1.1–1.6 尾链 ----
            p = setup(
                &mut layers,
                L::Hills64,
                map_hills,
                1000,
                Some(L::Zoom64 as usize),
                Some(L::Zoom64Hills as usize),
            );
            p = setup(&mut layers, L::Zoom32, map_zoom, 1000, Some(p), None);
            p = setup(&mut layers, L::Land32, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Zoom16, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Shore16, map_shore, 1000, Some(p), None);
            p = setup(&mut layers, L::SwampRiver16, map_swamp_river, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom8, map_zoom, 1002, Some(p), None);
            p = setup(&mut layers, L::Zoom4, map_zoom, 1003, Some(p), None);
            if large_biomes {
                p = setup(&mut layers, L::ZoomLargeA, map_zoom, 1004, Some(p), None);
                p = setup(&mut layers, L::ZoomLargeB, map_zoom, 1005, Some(p), None);
            }
            setup(&mut layers, L::Smooth4, map_smooth, 1000, Some(p), None);

            // 河流链
            p = setup(
                &mut layers,
                L::Zoom128River,
                map_zoom,
                1000,
                Some(L::RiverInit256 as usize),
                None,
            );
            p = setup(&mut layers, L::Zoom64River, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Zoom32River, map_zoom, 1002, Some(p), None);
            p = setup(&mut layers, L::Zoom16River, map_zoom, 1003, Some(p), None);
            p = setup(&mut layers, L::Zoom8River, map_zoom, 1004, Some(p), None);
            p = setup(&mut layers, L::Zoom4River, map_zoom, 1005, Some(p), None);
            if large_biomes {
                p = setup(&mut layers, L::ZoomLRiverA, map_zoom, 1006, Some(p), None);
                p = setup(&mut layers, L::ZoomLRiverB, map_zoom, 1007, Some(p), None);
            }
            p = setup(&mut layers, L::River4, map_river, 1, Some(p), None);
            setup(&mut layers, L::Smooth4River, map_smooth, 1000, Some(p), None);
        } else {
            // ---- 1.7+ 尾链 ----
            p = setup(
                &mut layers,
                L::Hills64,
                map_hills,
                1000,
                Some(L::BiomeEdge64 as usize),
                Some(L::Zoom64Hills as usize),
            );
            p = setup(&mut layers, L::Sunflower64, map_sunflower, 1001, Some(p), None);
            p = setup(&mut layers, L::Zoom32, map_zoom, 1000, Some(p), None);
            p = setup(&mut layers, L::Land32, map_land_fn, 3, Some(p), None);
            p = setup(&mut layers, L::Zoom16, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Shore16, map_shore, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom8, map_zoom, 1002, Some(p), None);
            p = setup(&mut layers, L::Zoom4, map_zoom, 1003, Some(p), None);
            if large_biomes {
                p = setup(&mut layers, L::ZoomLargeA, map_zoom, 1004, Some(p), None);
                p = setup(&mut layers, L::ZoomLargeB, map_zoom, 1005, Some(p), None);
            }
            setup(&mut layers, L::Smooth4, map_smooth, 1000, Some(p), None);

            // 河流链
            p = setup(
                &mut layers,
                L::Zoom128River,
                map_zoom,
                1000,
                Some(L::RiverInit256 as usize),
                None,
            );
            p = setup(&mut layers, L::Zoom64River, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Zoom32River, map_zoom, 1000, Some(p), None);
            p = setup(&mut layers, L::Zoom16River, map_zoom, 1001, Some(p), None);
            p = setup(&mut layers, L::Zoom8River, map_zoom, 1002, Some(p), None);
            p = setup(&mut layers, L::Zoom4River, map_zoom, 1003, Some(p), None);
            if large_biomes && mc == McVersion::V1_7 {
                p = setup(&mut layers, L::ZoomLRiverA, map_zoom, 1004, Some(p), None);
                p = setup(&mut layers, L::ZoomLRiverB, map_zoom, 1005, Some(p), None);
            }
            p = setup(&mut layers, L::River4, map_river, 1, Some(p), None);
            setup(&mut layers, L::Smooth4River, map_smooth, 1000, Some(p), None);
        }

        setup(
            &mut layers,
            L::RiverMix4,
            map_river_mix,
            100,
            Some(L::Smooth4 as usize),
            Some(L::Smooth4River as usize),
        );

        let mut entry_4 = L::RiverMix4 as usize;
        if mc >= McVersion::V1_13 {
            // 1.13+ 海洋变体链
            let mut q = setup(&mut layers, L::OceanTemp256, map_ocean_temp, 2, None, None);
            q = setup(&mut layers, L::Zoom128Ocean, map_zoom, 2001, Some(q), None);
            q = setup(&mut layers, L::Zoom64Ocean, map_zoom, 2002, Some(q), None);
            q = setup(&mut layers, L::Zoom32Ocean, map_zoom, 2003, Some(q), None);
            q = setup(&mut layers, L::Zoom16Ocean, map_zoom, 2004, Some(q), None);
            q = setup(&mut layers, L::Zoom8Ocean, map_zoom, 2005, Some(q), None);
            q = setup(&mut layers, L::Zoom4Ocean, map_zoom, 2006, Some(q), None);
            setup(
                &mut layers,
                L::OceanMix4,
                map_ocean_mix,
                100,
                Some(L::RiverMix4 as usize),
                Some(q),
            );
            entry_4 = L::OceanMix4 as usize;
            if mc <= McVersion::V1_14 {
                setup(
                    &mut layers,
                    L::Voronoi1,
                    map_voronoi114,
                    10,
                    Some(entry_4),
                    None,
                );
            } else {
                setup(
                    &mut layers,
                    L::Voronoi1,
                    map_voronoi,
                    LAYER_INIT_SHA,
                    Some(entry_4),
                    None,
                );
            }
        } else {
            setup(
                &mut layers,
                L::Voronoi1,
                map_voronoi114,
                10,
                Some(L::RiverMix4 as usize),
                None,
            );
        }

        // 非官方入口（最新可用层），与 `setupLayerStack` 末尾一致
        let (entry_16, entry_64, entry_256) = if large_biomes {
            (
                L::Zoom4 as usize,
                if mc <= McVersion::V1_6 {
                    L::SwampRiver16 as usize
                } else {
                    L::Shore16 as usize
                },
                if mc <= McVersion::V1_6 {
                    L::Hills64 as usize
                } else {
                    L::Sunflower64 as usize
                },
            )
        } else if mc >= McVersion::V1_1 {
            (
                if mc <= McVersion::V1_6 {
                    L::SwampRiver16 as usize
                } else {
                    L::Shore16 as usize
                },
                if mc <= McVersion::V1_6 {
                    L::Hills64 as usize
                } else {
                    L::Sunflower64 as usize
                },
                if mc <= McVersion::V1_14 {
                    L::Biome256 as usize
                } else {
                    L::Bamboo256 as usize
                },
            )
        } else {
            // B1.8/1.0：没有 hills/swampRiver 层
            (
                L::Zoom16 as usize,
                L::Zoom64 as usize,
                L::Biome256 as usize,
            )
        };

        LayerStack {
            layers,
            entry_1: L::Voronoi1 as usize,
            entry_4,
            entry_16,
            entry_64,
            entry_256,
            // 1.13- 不使用；占位值（set_world_seed 会按种子重新初始化）
            ocean_rnd: PerlinNoise::new_java(&mut JavaRandom::new(0)),
            viable_filter: std::cell::Cell::new(None),
            viable_failed: std::cell::Cell::new(false),
        }
    }

    /// `setLayerSeed`（从 entry_1 递归等价于对每层独立计算）：注入世界种子。
    pub fn set_world_seed(&mut self, world_seed: u64) {
        for l in &mut self.layers {
            let ls = l.layer_salt;
            if ls == 0 {
                // 1.12- 的 hills zoom 分支保持零初始化
                l.start_salt = 0;
                l.start_seed = 0;
            } else if ls == LAYER_INIT_SHA {
                // 1.15+ voronoi 用 SHA-256 初始化
                l.start_salt = get_voronoi_sha(world_seed);
                l.start_seed = 0;
            } else {
                let st = start_salt(world_seed, ls);
                l.start_salt = st;
                l.start_seed = step_seed(st, 0);
            }
        }
        // 1.13+ 才构建海洋温度链（该槽位的 `mc` 仅在被 `setup` 初始化后
        // 才是真实版本，默认槽位为 V1_7），对应 C 的 `layer->noise != NULL`
        if self.layers[LayerId::OceanTemp256 as usize].mc >= McVersion::V1_13 {
            // `perlinInit(&oceanRnd, worldSeed)`
            self.ocean_rnd = PerlinNoise::new_java(&mut JavaRandom::new(world_seed as i64));
        }
    }

    /// `getLayerForScale`：按目标比例取入口层（1/4/16/64/256）。
    pub fn entry_for_scale(&self, scale: i32) -> Option<usize> {
        Some(match scale {
            1 => self.entry_1,
            4 => self.entry_4,
            16 => self.entry_16,
            64 => self.entry_64,
            256 => self.entry_256,
            _ => return Option::None,
        })
    }

    /// `genArea`：从入口层生成 `w*h` 的区域。
    pub fn gen_area(&self, entry: usize, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        let mut out = vec![0i32; (w * h) as usize];
        get_map(self, entry, &mut out, x, z, w, h);
        out
    }

    /// `setupGenerator` 的 `FORCE_OCEAN_VARIANTS` 分支（generator.c 的
    /// `mapOceanMixMod` 接线）：1.13+ 的海洋变体在 scale 16/64/256 入口
    /// 生效（scale 1/4 不受该标志影响），把对应入口的陆地输出与海洋温度
    /// 链按 [`map_ocean_mix_mod`] 混合。
    ///
    /// # Panics
    ///
    /// `mc < 1.13`（海洋链未构建）或 `scale` 不是 16/64/256。
    pub fn gen_area_ocean_mix_mod(&self, scale: i32, x: i32, z: i32, w: i32, h: i32) -> Vec<i32> {
        assert!(
            self.layers[LayerId::OceanTemp256 as usize].mc >= McVersion::V1_13,
            "FORCE_OCEAN_VARIANTS 要求 mc >= 1.13"
        );
        let ocean_entry = match scale {
            16 => LayerId::Zoom16Ocean as usize,
            64 => LayerId::Zoom64Ocean as usize,
            256 => LayerId::OceanTemp256 as usize,
            _ => panic!("FORCE_OCEAN_VARIANTS 只替换 scale 16/64/256 的入口层"),
        };
        let land_entry = self.entry_for_scale(scale).unwrap();
        let land = self.gen_area(land_entry, x, z, w, h);
        let ocean = self.gen_area(ocean_entry, x, z, w, h);
        map_ocean_mix_mod(&land, &ocean)
    }
}

/// `genBiomes` 的旧版主世界分支（mc B1.8–1.17, `DIM_OVERWORLD`）。
///
/// 旧版主世界没有垂直噪声：C 中 `y` 被完全忽略，生成 2D 平面后沿 y
/// 逐层复制（`for (k = 1; k < r.sy; k++) memcpy`），这里同样处理。
///
/// # Panics
///
/// `scale` 不是 1/4/16/64/256（C 中 `getLayerForScale` 返回 NULL）。
pub fn gen_biomes(ls: &LayerStack, r: Range) -> Vec<BiomeId> {
    let entry = ls
        .entry_for_scale(r.scale)
        .unwrap_or_else(|| panic!("Generator: 旧版主世界不支持的 scale {}", r.scale));
    let plane = ls.gen_area(entry, r.x, r.z, r.sx, r.sz);
    let sy = if r.sy == 0 { 1 } else { r.sy };
    let mut out = Vec::with_capacity((r.sx * r.sz * sy) as usize);
    for _ in 0..sy {
        out.extend(plane.iter().map(|&v| {
            BiomeId::from_i32(v)
                .unwrap_or_else(|| panic!("Generator: 分层群系源产生未知群系 ID {v}"))
        }));
    }
    out
}

/// 调用某层的 map 函数（对应 C `l->getMap(l, ...)`）。
#[inline]
fn get_map(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    (l.map)(ls, idx, out, x, z, w, h);
    // 结构可行性剪枝（C 的 mapViableBiome/mapViableShore 替换层行为）：
    // 粗层区域不含目标群系时，C 的 getMap 链返回 err，本次查询判失败。
    // 这里不中断计算（结果反正作废），只记录失败标记，效果等价。
    if let Some((styp, mc)) = ls.viable_filter.get() {
        if ls.viable_failed.get() {
            return;
        }
        let cells = &out[..(w * h) as usize];
        let ok = if idx == LayerId::Biome256 as usize {
            crate::structure::viability::viable_biome_area_ok(styp, cells)
        } else if idx == LayerId::Shore16 as usize {
            crate::structure::viability::viable_shore_area_ok(styp, mc, cells)
        } else {
            true
        };
        if !ok {
            ls.viable_failed.set(true);
        }
    }
}

/// 生成 1 格边界的父层区域（多数层的公共前缀），返回 `(缓冲, 行宽)`。
fn parent_area(ls: &LayerStack, l: &Layer, x: i32, z: i32, w: i32, h: i32) -> (Vec<i32>, i32) {
    let (pw, ph) = (w + 2, h + 2);
    let mut buf = vec![0i32; (pw * ph) as usize];
    get_map(ls, l.p.unwrap(), &mut buf, x - 1, z - 1, pw, ph);
    (buf, pw)
}

/// `isAny4`
#[inline]
fn is_any4(id: i32, a: i32, b: i32, c: i32, d: i32) -> bool {
    id == a || id == b || id == c || id == d
}

// ============================================================================
// 层函数（按 cubiomes `layers.c` 顺序）
// ============================================================================

/// `mapContinent`：1:4096 大陆初始化（1/10 概率为陆地，原点强制为陆地）。
fn map_continent(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let ss = ls.layers[idx].start_seed;
    for j in 0..h {
        for i in 0..w {
            let cs = chunk_seed(ss, i + x, j + z);
            out[(j * w + i) as usize] = first_is_zero(cs, 10) as i32;
        }
    }
    if x > -w && x <= 0 && z > -h && z <= 0 {
        out[(-z * w - x) as usize] = 1;
    }
}

/// `mapZoom`/`mapZoomFuzzy` 的公共实现（fuzzy 模式下最后一格纯随机）。
#[allow(clippy::too_many_arguments)]
fn zoom_impl(
    ls: &LayerStack,
    idx: usize,
    out: &mut [i32],
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    fuzzy: bool,
) {
    let l = &ls.layers[idx];
    let px = x >> 1;
    let pz = z >> 1;
    let pw = ((x + w) >> 1) - px + 1;
    let ph = ((z + h) >> 1) - pz + 1;

    // 多分配一行 + 1 格：C 循环在 i=pw-1, j=ph-1 时会读到 pW*(pH+1)
    // （越界读入暂存区，其值只影响永远不会被拷贝到输出的边缘格，见模块文档）
    let mut pbuf = vec![0i32; (pw * (ph + 1) + 1) as usize];
    get_map(ls, l.p.unwrap(), &mut pbuf[..(pw * ph) as usize], px, pz, pw, ph);

    let new_w = pw * 2;
    let mut zbuf = vec![0i32; (new_w * 2 * ph) as usize];

    let st = l.start_salt as u32;
    let ss = l.start_seed as u32;
    let nw = new_w as usize;

    for j in 0..ph {
        let mut idx2 = (j * 2) as usize * nw;
        let mut v00 = pbuf[(j * pw) as usize];
        let mut v01 = pbuf[((j + 1) * pw) as usize];

        for i in 0..pw {
            let v10 = pbuf[(i + 1 + j * pw) as usize];
            let v11 = pbuf[(i + 1 + (j + 1) * pw) as usize];

            if v00 == v01 && v00 == v10 && v00 == v11 {
                zbuf[idx2] = v00;
                zbuf[idx2 + 1] = v00;
                zbuf[idx2 + nw] = v00;
                zbuf[idx2 + nw + 1] = v00;
                idx2 += 2;
                v00 = v10;
                v01 = v11;
                continue;
            }

            let chunk_x = (i + px) * 2;
            let chunk_z = (j + pz) * 2;

            let mut cs = ss;
            cs = cs.wrapping_add(chunk_x as u32);
            cs = cs.wrapping_mul(cs.wrapping_mul(1284865837).wrapping_add(4150755663));
            cs = cs.wrapping_add(chunk_z as u32);
            cs = cs.wrapping_mul(cs.wrapping_mul(1284865837).wrapping_add(4150755663));
            cs = cs.wrapping_add(chunk_x as u32);
            cs = cs.wrapping_mul(cs.wrapping_mul(1284865837).wrapping_add(4150755663));
            cs = cs.wrapping_add(chunk_z as u32);

            zbuf[idx2] = v00;
            zbuf[idx2 + nw] = if (cs >> 24) & 1 != 0 { v01 } else { v00 };
            idx2 += 1;

            cs = cs.wrapping_mul(cs.wrapping_mul(1284865837).wrapping_add(4150755663));
            cs = cs.wrapping_add(st);
            zbuf[idx2] = if (cs >> 24) & 1 != 0 { v10 } else { v00 };

            zbuf[idx2 + nw] = if fuzzy {
                cs = cs.wrapping_mul(cs.wrapping_mul(1284865837).wrapping_add(4150755663));
                cs = cs.wrapping_add(st);
                match (cs >> 24) & 3 {
                    0 => v00,
                    1 => v10,
                    2 => v01,
                    _ => v11,
                }
            } else {
                select4(cs, st, v00, v01, v10, v11)
            };
            idx2 += 1;

            v00 = v10;
            v01 = v11;
        }
    }

    let (xo, zo) = ((x & 1) as usize, (z & 1) as usize);
    for j in 0..h as usize {
        let src = (j + zo) * nw + xo;
        out[j * w as usize..(j + 1) * w as usize]
            .copy_from_slice(&zbuf[src..src + w as usize]);
    }
}

/// `select4`：mapZoom 最后一格的多数投票（平票时随机）。
fn select4(mut cs: u32, st: u32, v00: i32, v01: i32, v10: i32, v11: i32) -> i32 {
    let cv00 = (v00 == v10) as i32 + (v00 == v01) as i32 + (v00 == v11) as i32;
    let cv10 = (v10 == v01) as i32 + (v10 == v11) as i32;
    let cv01 = (v01 == v11) as i32;
    if cv00 > cv10 && cv00 > cv01 {
        v00
    } else if cv10 > cv00 {
        v10
    } else if cv01 > cv00 {
        v01
    } else {
        cs = cs.wrapping_mul(cs.wrapping_mul(1284865837).wrapping_add(4150755663));
        cs = cs.wrapping_add(st);
        match (cs >> 24) & 3 {
            0 => v00,
            1 => v10,
            2 => v01,
            _ => v11,
        }
    }
}

/// `mapZoomFuzzy`：首个 zoom（1:4096 → 1:2048）。
fn map_zoom_fuzzy(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    zoom_impl(ls, idx, out, x, z, w, h, true)
}

/// `mapZoom`：常规 zoom 层。
fn map_zoom(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    zoom_impl(ls, idx, out, x, z, w, h, false)
}

/// `mapLand`（1.7+ `mapAddIsland` 等价物）。
///
/// 注意 C 代码中 `case forest:` 与 `v != forest` 的 `forest`（4）在此阶段
/// 实际匹配温度分类 `Freezing`（同为 4），按原样保留。
fn map_land(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let (st, ss) = (l.start_salt, l.start_seed);
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        let mut v00 = pbuf[row0];
        let mut vt0 = pbuf[row0 + 1];
        let mut v02 = pbuf[row2];
        let mut vt2 = pbuf[row2 + 1];

        for i in 0..w as usize {
            let v11 = pbuf[row1 + i + 1];
            let v20 = pbuf[row0 + i + 2];
            let v22 = pbuf[row2 + i + 2];
            let mut v = v11;

            match v11 {
                OCEAN => {
                    if v00 != 0 || v20 != 0 || v02 != 0 || v22 != 0 {
                        // 四角有非海洋：竞争生长陆地
                        let mut cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                        let mut inc = 0;
                        v = 1;

                        if v00 != OCEAN {
                            inc += 1;
                            v = v00;
                            cs = step_seed(cs, st);
                        }
                        if v20 != OCEAN {
                            inc += 1;
                            if inc == 1 || first_is_zero(cs, 2) {
                                v = v20;
                            }
                            cs = step_seed(cs, st);
                        }
                        if v02 != OCEAN {
                            inc += 1;
                            match inc {
                                1 => v = v02,
                                2 => {
                                    if first_is_zero(cs, 2) {
                                        v = v02;
                                    }
                                }
                                _ => {
                                    if first_is_zero(cs, 3) {
                                        v = v02;
                                    }
                                }
                            }
                            cs = step_seed(cs, st);
                        }
                        if v22 != OCEAN {
                            inc += 1;
                            match inc {
                                1 => v = v22,
                                2 => {
                                    if first_is_zero(cs, 2) {
                                        v = v22;
                                    }
                                }
                                3 => {
                                    if first_is_zero(cs, 3) {
                                        v = v22;
                                    }
                                }
                                _ => {
                                    if first_is_zero(cs, 4) {
                                        v = v22;
                                    }
                                }
                            }
                            cs = step_seed(cs, st);
                        }

                        if v != FOREST && !first_is_zero(cs, 3) {
                            v = OCEAN;
                        }
                    }
                }
                FOREST => {}
                _ => {
                    if v00 == 0 || v20 == 0 || v02 == 0 || v22 == 0 {
                        let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                        if first_is_zero(cs, 5) {
                            v = 0;
                        }
                    }
                }
            }

            out[i + j * w as usize] = v;
            v00 = vt0;
            vt0 = v20;
            v02 = vt2;
            vt2 = v22;
        }
    }
}

/// `mapLand16`（1.0–1.6 的 `mapAddIsland` 等价物）。
///
/// 与 1.7+ 的 [`map_land`] 逻辑同构，差异：
/// - 竞争生长的"陆地"值固定为 1（此阶段没有温度分类）；
/// - 海洋化判定结果为 `ocean`（若原值是 `snowy_tundra` 则 `frozen_ocean`），
///   而非恒 `ocean`；没有 `v != forest`（温度分类 Freezing）的特判。
fn map_land16(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let (st, ss) = (l.start_salt, l.start_seed);
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        let mut v00 = pbuf[row0];
        let mut vt0 = pbuf[row0 + 1];
        let mut v02 = pbuf[row2];
        let mut vt2 = pbuf[row2 + 1];

        for i in 0..w as usize {
            let v11 = pbuf[row1 + i + 1];
            let v20 = pbuf[row0 + i + 2];
            let v22 = pbuf[row2 + i + 2];
            let mut v = v11;

            if v11 != 0 || (v00 == 0 && v20 == 0 && v02 == 0 && v22 == 0) {
                // 陆地格（或全海区域）：四角有海洋时 1/5 概率缩小为海洋
                if v11 != 0 && (v00 == 0 || v20 == 0 || v02 == 0 || v22 == 0) {
                    let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                    if first_is_zero(cs, 5) {
                        v = if v == SNOWY_TUNDRA { FROZEN_OCEAN } else { OCEAN };
                    }
                }
            } else {
                // 海洋格且四角有陆地：竞争生长（与 map_land 相同的抽签序列）
                let mut cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                let mut inc = 0;
                v = 1;

                if v00 != OCEAN {
                    inc += 1;
                    v = v00;
                    cs = step_seed(cs, st);
                }
                if v20 != OCEAN {
                    inc += 1;
                    if inc == 1 || first_is_zero(cs, 2) {
                        v = v20;
                    }
                    cs = step_seed(cs, st);
                }
                if v02 != OCEAN {
                    inc += 1;
                    match inc {
                        1 => v = v02,
                        2 => {
                            if first_is_zero(cs, 2) {
                                v = v02;
                            }
                        }
                        _ => {
                            if first_is_zero(cs, 3) {
                                v = v02;
                            }
                        }
                    }
                    cs = step_seed(cs, st);
                }
                if v22 != OCEAN {
                    inc += 1;
                    match inc {
                        1 => v = v22,
                        2 => {
                            if first_is_zero(cs, 2) {
                                v = v22;
                            }
                        }
                        3 => {
                            if first_is_zero(cs, 3) {
                                v = v22;
                            }
                        }
                        _ => {
                            if first_is_zero(cs, 4) {
                                v = v22;
                            }
                        }
                    }
                    cs = step_seed(cs, st);
                }

                if !first_is_zero(cs, 3) {
                    v = if v == SNOWY_TUNDRA { FROZEN_OCEAN } else { OCEAN };
                }
            }

            out[i + j * w as usize] = v;
            v00 = vt0;
            vt0 = v20;
            v02 = vt2;
            vt2 = v22;
        }
    }
}

/// `mapLandB18`（Beta 1.8 的陆地扩张/收缩，0/1 二值）。
fn map_land_b18(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let ss = l.start_seed;
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        let mut v00 = pbuf[row0];
        let mut vt0 = pbuf[row0 + 1];
        let mut v02 = pbuf[row2];
        let mut vt2 = pbuf[row2 + 1];

        for i in 0..w as usize {
            let v11 = pbuf[row1 + i + 1];
            let v20 = pbuf[row0 + i + 2];
            let v22 = pbuf[row2 + i + 2];
            let mut v = v11;

            if v11 == 0 && (v00 != 0 || v02 != 0 || v20 != 0 || v22 != 0) {
                // 海洋邻接陆地：1/3 概率扩张（firstInt(3)/2 → 0 或 1）
                let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                v = first_int(cs, 3) / 2;
            } else if v11 == 1 && (v00 != 1 || v02 != 1 || v20 != 1 || v22 != 1) {
                // 陆地邻接海洋：1/5 概率收缩（1 - firstInt(5)/4）
                let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                v = 1 - first_int(cs, 5) / 4;
            }

            out[i + j * w as usize] = v;
            v00 = vt0;
            vt0 = v20;
            v02 = vt2;
            vt2 = v22;
        }
    }
}

/// `mapSnow16`（1.0–1.6）：陆地按 1/5 概率标记为 `snowy_tundra`，否则
/// `plains`（海洋格保持 `ocean`）。
fn map_snow16(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let ss = l.start_seed;
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let mut v11 = pbuf[i + 1 + (j + 1) * pwu];
            if v11 != OCEAN {
                let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                v11 = if first_is_zero(cs, 5) { SNOWY_TUNDRA } else { PLAINS };
            }
            out[i + j * w as usize] = v11;
        }
    }
}

/// `mapSwampRiver`（1.1–1.6）：沼泽 1/6、丛林（含丛林丘陵）1/8 概率变河流。
fn map_swamp_river(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p.unwrap(), out, x, z, w, h);
    let ss = l.start_seed;

    for j in 0..h {
        for i in 0..w {
            let o = (i + j * w) as usize;
            let v = out[o];
            if v != SWAMP && v != JUNGLE && v != JUNGLE_HILLS {
                continue;
            }
            let cs = chunk_seed(ss, i + x, j + z);
            if first_is_zero(cs, if v == SWAMP { 6 } else { 8 }) {
                out[o] = RIVER;
            }
        }
    }
}

/// `mapIsland`（`mapRemoveTooMuchOcean`）：被海洋包围的深海格 1/2 概率变陆地。
fn map_island(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let ss = l.start_seed;
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let v11 = pbuf[i + 1 + (j + 1) * pwu];
            out[i + j * w as usize] = v11;

            if v11 == CAT_OCEANIC
                && pbuf[i + 1 + j * pwu] == CAT_OCEANIC
                && pbuf[i + 2 + (j + 1) * pwu] == CAT_OCEANIC
                && pbuf[i + (j + 1) * pwu] == CAT_OCEANIC
                && pbuf[i + 1 + (j + 2) * pwu] == CAT_OCEANIC
            {
                let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                if first_is_zero(cs, 2) {
                    out[i + j * w as usize] = 1;
                }
            }
        }
    }
}

/// `mapSnow`（`mapAddSnow`）：陆地按 6 面骰分配 Freezing/Cold/Warm 分类。
fn map_snow(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let ss = l.start_seed;
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let mut v11 = pbuf[i + 1 + (j + 1) * pwu];
            if !is_shallow_ocean(v11) {
                let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                let r = first_int(cs, 6);
                if r == 0 {
                    v11 = CAT_FREEZING;
                } else if r <= 1 {
                    v11 = CAT_COLD;
                } else {
                    v11 = CAT_WARM;
                }
            }
            out[i + j * w as usize] = v11;
        }
    }
}

/// `mapCool`（`mapCoolWarm`）：Warm 邻接 Cold/Freezing 时降级为 Lush。
fn map_cool(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let mut v11 = pbuf[i + 1 + (j + 1) * pwu];
            if v11 == CAT_WARM {
                let v10 = pbuf[i + 1 + j * pwu];
                let v21 = pbuf[i + 2 + (j + 1) * pwu];
                let v01 = pbuf[i + (j + 1) * pwu];
                let v12 = pbuf[i + 1 + (j + 2) * pwu];
                if is_any4(CAT_COLD, v10, v21, v01, v12) || is_any4(CAT_FREEZING, v10, v21, v01, v12)
                {
                    v11 = CAT_LUSH;
                }
            }
            out[i + j * w as usize] = v11;
        }
    }
}

/// `mapHeat`（`mapHeatIce`）：Freezing 邻接 Warm/Lush 时升级为 Cold。
fn map_heat(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let mut v11 = pbuf[i + 1 + (j + 1) * pwu];
            if v11 == CAT_FREEZING {
                let v10 = pbuf[i + 1 + j * pwu];
                let v21 = pbuf[i + 2 + (j + 1) * pwu];
                let v01 = pbuf[i + (j + 1) * pwu];
                let v12 = pbuf[i + 1 + (j + 2) * pwu];
                if is_any4(CAT_WARM, v10, v21, v01, v12) || is_any4(CAT_LUSH, v10, v21, v01, v12) {
                    v11 = CAT_COLD;
                }
            }
            out[i + j * w as usize] = v11;
        }
    }
}

/// `mapSpecial`：1/13 概率给陆地打上 0x100..0xf00 的高位标记（稀有变体）。
fn map_special(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p.unwrap(), out, x, z, w, h);
    let (st, ss) = (l.start_salt, l.start_seed);

    for j in 0..h {
        for i in 0..w {
            let o = (i + j * w) as usize;
            let mut v = out[o];
            if v == CAT_OCEANIC {
                continue;
            }
            let mut cs = chunk_seed(ss, i + x, j + z);
            if first_is_zero(cs, 13) {
                cs = step_seed(cs, st);
                v |= ((1 + first_int(cs, 15)) << 8) & 0xf00;
                out[o] = v;
            }
        }
    }
}

/// `mapMushroom`（`mapAddMushroomIsland`）：四角皆海洋的洋中格 1/100 变蘑菇岛。
fn map_mushroom(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let ss = l.start_seed;
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let mut v11 = pbuf[i + 1 + (j + 1) * pwu];
            if v11 == 0
                && pbuf[i + j * pwu] == 0
                && pbuf[i + 2 + j * pwu] == 0
                && pbuf[i + (j + 2) * pwu] == 0
                && pbuf[i + 2 + (j + 2) * pwu] == 0
            {
                let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                if first_is_zero(cs, 100) {
                    v11 = MUSHROOM_FIELDS;
                }
            }
            out[i + j * w as usize] = v11;
        }
    }
}

/// `mapDeepOcean`：四邻皆为浅海的浅海格变为对应深海。
fn map_deep_ocean(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let pwu = pw as usize;

    for j in 0..h as usize {
        for i in 0..w as usize {
            let mut v11 = pbuf[i + 1 + (j + 1) * pwu];
            if is_shallow_ocean(v11) {
                let mut oceans = 0;
                if is_shallow_ocean(pbuf[i + 1 + j * pwu]) {
                    oceans += 1;
                }
                if is_shallow_ocean(pbuf[i + 2 + (j + 1) * pwu]) {
                    oceans += 1;
                }
                if is_shallow_ocean(pbuf[i + (j + 1) * pwu]) {
                    oceans += 1;
                }
                if is_shallow_ocean(pbuf[i + 1 + (j + 2) * pwu]) {
                    oceans += 1;
                }
                if oceans >= 4 {
                    v11 = match v11 {
                        WARM_OCEAN => DEEP_WARM_OCEAN,
                        LUKEWARM_OCEAN => DEEP_LUKEWARM_OCEAN,
                        OCEAN => DEEP_OCEAN,
                        COLD_OCEAN => DEEP_COLD_OCEAN,
                        FROZEN_OCEAN => DEEP_FROZEN_OCEAN,
                        _ => DEEP_OCEAN,
                    };
                }
            }
            out[i + j * w as usize] = v11;
        }
    }
}

// `mapBiome` 的分类群系表
const WARM_BIOMES: [i32; 6] = [DESERT, DESERT, DESERT, SAVANNA, SAVANNA, PLAINS];
const LUSH_BIOMES: [i32; 6] = [FOREST, DARK_FOREST, MOUNTAINS, PLAINS, BIRCH_FOREST, SWAMP];
const COLD_BIOMES: [i32; 4] = [FOREST, MOUNTAINS, TAIGA, PLAINS];
const SNOW_BIOMES: [i32; 4] = [SNOWY_TUNDRA, SNOWY_TUNDRA, SNOWY_TUNDRA, SNOWY_TAIGA];

// `mapBiome` 的 1.6- 旧群系表（`oldBiomes` / `oldBiomes11`）
const OLD_BIOMES: [i32; 7] = [DESERT, FOREST, MOUNTAINS, SWAMP, PLAINS, TAIGA, JUNGLE];
const OLD_BIOMES_11: [i32; 6] = [DESERT, FOREST, MOUNTAINS, SWAMP, PLAINS, TAIGA];

/// `mapBiome`：温度分类（1.7+）或 0/1/plains/snowy_tundra（1.6-）→ 具体群系。
///
/// 1.6- 分支（C `mc <= MC_1_6`）：1.1- 用 6 项表，1.2+ 用 7 项表（含丛林）；
/// 非 `plains` 输入（即 `snowy_tundra`）强制回到 `snowy_tundra`，例外是
/// 1.3+ 抽中 `taiga` 时保留。
fn map_biome(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let mc = l.mc;
    get_map(ls, l.p.unwrap(), out, x, z, w, h);
    let ss = l.start_seed;

    for j in 0..h {
        for i in 0..w {
            let o = (i + j * w) as usize;
            let id = out[o];
            let has_high_bit = id & 0xf00;
            let id = id & !0xf00;

            if mc <= McVersion::V1_6 {
                if id == OCEAN || id == MUSHROOM_FIELDS {
                    out[o] = id;
                    continue;
                }
                let cs = chunk_seed(ss, i + x, j + z);
                let mut v = if mc <= McVersion::V1_1 {
                    OLD_BIOMES_11[first_int(cs, 6) as usize]
                } else {
                    OLD_BIOMES[first_int(cs, 7) as usize]
                };
                if id != PLAINS && (v != TAIGA || mc <= McVersion::V1_2) {
                    v = SNOWY_TUNDRA;
                }
                out[o] = v;
                continue;
            }

            if is_oceanic(id) || id == MUSHROOM_FIELDS {
                out[o] = id;
                continue;
            }

            let cs = chunk_seed(ss, i + x, j + z);
            let v = match id {
                CAT_WARM => {
                    if has_high_bit != 0 {
                        if first_is_zero(cs, 3) {
                            BADLANDS_PLATEAU
                        } else {
                            WOODED_BADLANDS_PLATEAU
                        }
                    } else {
                        WARM_BIOMES[first_int(cs, 6) as usize]
                    }
                }
                CAT_LUSH => {
                    if has_high_bit != 0 {
                        JUNGLE
                    } else {
                        LUSH_BIOMES[first_int(cs, 6) as usize]
                    }
                }
                CAT_COLD => {
                    if has_high_bit != 0 {
                        GIANT_TREE_TAIGA
                    } else {
                        COLD_BIOMES[first_int(cs, 4) as usize]
                    }
                }
                CAT_FREEZING => SNOW_BIOMES[first_int(cs, 4) as usize],
                _ => MUSHROOM_FIELDS,
            };
            out[o] = v;
        }
    }
}

/// `mapNoise`（`mapRiverInit`）：河流噪声初始化（1.7+ 模 299999，1.6- 模 2）。
fn map_noise(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p.unwrap(), out, x, z, w, h);
    let ss = l.start_seed;
    let m = if l.mc <= V1_6 { 2 } else { 299999 };

    for j in 0..h {
        for i in 0..w {
            let o = (i + j * w) as usize;
            if out[o] > 0 {
                let cs = chunk_seed(ss, i + x, j + z);
                out[o] = first_int(cs, m) + 2;
            } else {
                out[o] = 0;
            }
        }
    }
}

/// `mapBamboo`（1.14+ `mapAddBamboo`）：丛林 1/10 变竹林。
fn map_bamboo(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p.unwrap(), out, x, z, w, h);
    let ss = l.start_seed;

    for j in 0..h {
        for i in 0..w {
            let o = (i + j * w) as usize;
            if out[o] != JUNGLE {
                continue;
            }
            let cs = chunk_seed(ss, i + x, j + z);
            if first_is_zero(cs, 10) {
                out[o] = BAMBOO_JUNGLE;
            }
        }
    }
}

/// `replaceEdge`
#[allow(clippy::too_many_arguments)]
fn replace_edge(
    out: &mut [i32],
    idx: usize,
    mc: McVersion,
    v10: i32,
    v21: i32,
    v01: i32,
    v12: i32,
    id: i32,
    base: i32,
    edge: i32,
) -> bool {
    if id != base {
        return false;
    }
    if are_similar(mc, v10, base)
        && are_similar(mc, v21, base)
        && are_similar(mc, v01, base)
        && are_similar(mc, v12, base)
    {
        out[idx] = id;
    } else {
        out[idx] = edge;
    }
    true
}

/// `mapBiomeEdge`：恶地高原/巨型针叶林的边缘退化与沙漠/沼泽边界特判。
fn map_biome_edge(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let mc = l.mc;
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        for i in 0..w as usize {
            let v11 = pbuf[row1 + i + 1];
            let v10 = pbuf[row0 + i + 1];
            let v21 = pbuf[row1 + i + 2];
            let v01 = pbuf[row1 + i];
            let v12 = pbuf[row2 + i + 1];
            let o = i + j * w as usize;

            if !replace_edge(out, o, mc, v10, v21, v01, v12, v11, WOODED_BADLANDS_PLATEAU, BADLANDS)
                && !replace_edge(out, o, mc, v10, v21, v01, v12, v11, BADLANDS_PLATEAU, BADLANDS)
                && !replace_edge(out, o, mc, v10, v21, v01, v12, v11, GIANT_TREE_TAIGA, TAIGA)
            {
                if v11 == DESERT {
                    out[o] = if !is_any4(SNOWY_TUNDRA, v10, v21, v01, v12) {
                        v11
                    } else {
                        WOODED_MOUNTAINS
                    };
                } else if v11 == SWAMP {
                    if !is_any4(DESERT, v10, v21, v01, v12)
                        && !is_any4(SNOWY_TAIGA, v10, v21, v01, v12)
                        && !is_any4(SNOWY_TUNDRA, v10, v21, v01, v12)
                    {
                        if !is_any4(JUNGLE, v10, v21, v01, v12)
                            && !is_any4(BAMBOO_JUNGLE, v10, v21, v01, v12)
                        {
                            out[o] = v11;
                        } else {
                            out[o] = JUNGLE_EDGE;
                        }
                    } else {
                        out[o] = PLAINS;
                    }
                } else {
                    out[o] = v11;
                }
            }
        }
    }
}

/// `mapHills`：山丘与突变群系（双亲层：群系链 + 河流噪声链）。
fn map_hills(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (a, pw) = parent_area(ls, l, x, z, w, h);
    let (pwu, ph) = (pw as usize, h + 2);
    let mut riv = vec![0i32; (pw * ph) as usize];
    get_map(ls, l.p2.unwrap(), &mut riv, x - 1, z - 1, pw, ph);

    let mc = l.mc;
    let (st, ss) = (l.start_salt, l.start_seed);

    for j in 0..h as usize {
        for i in 0..w as usize {
            let a11 = a[i + 1 + (j + 1) * pwu]; // 群系分支
            let b11 = riv[i + 1 + (j + 1) * pwu]; // 河流分支
            let o = i + j * w as usize;
            let bn = if mc >= V1_7 { (b11 - 2) % 29 } else { -1 };

            if bn == 1 && b11 >= 2 && !is_shallow_ocean(a11) {
                let m = get_mutated(mc, a11);
                out[o] = if m > 0 { m } else { a11 };
                continue;
            }

            let mut cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
            if bn == 0 || first_is_zero(cs, 3) {
                let mut hill = a11;
                match a11 {
                    DESERT => hill = DESERT_HILLS,
                    FOREST => hill = WOODED_HILLS,
                    BIRCH_FOREST => hill = BIRCH_FOREST_HILLS,
                    DARK_FOREST => hill = PLAINS,
                    TAIGA => hill = TAIGA_HILLS,
                    GIANT_TREE_TAIGA => hill = GIANT_TREE_TAIGA_HILLS,
                    SNOWY_TAIGA => hill = SNOWY_TAIGA_HILLS,
                    PLAINS => {
                        if mc <= V1_6 {
                            hill = FOREST;
                        } else {
                            cs = step_seed(cs, st);
                            hill = if first_is_zero(cs, 3) { WOODED_HILLS } else { FOREST };
                        }
                    }
                    SNOWY_TUNDRA => hill = SNOWY_MOUNTAINS,
                    JUNGLE => hill = JUNGLE_HILLS,
                    BAMBOO_JUNGLE => hill = BAMBOO_JUNGLE_HILLS,
                    OCEAN => {
                        if mc >= V1_7 {
                            hill = DEEP_OCEAN;
                        }
                    }
                    MOUNTAINS => {
                        if mc >= V1_7 {
                            hill = WOODED_MOUNTAINS;
                        }
                    }
                    SAVANNA => hill = SAVANNA_PLATEAU,
                    _ => {
                        if are_similar(mc, a11, WOODED_BADLANDS_PLATEAU) {
                            hill = BADLANDS;
                        } else if is_deep_ocean(a11) {
                            cs = step_seed(cs, st);
                            if first_is_zero(cs, 3) {
                                cs = step_seed(cs, st);
                                hill = if first_is_zero(cs, 2) { PLAINS } else { FOREST };
                            }
                        }
                    }
                }

                if bn == 0 && hill != a11 {
                    hill = get_mutated(mc, hill);
                    if hill < 0 {
                        hill = a11;
                    }
                }

                if hill != a11 {
                    let a10 = a[i + 1 + j * pwu];
                    let a21 = a[i + 2 + (j + 1) * pwu];
                    let a01 = a[i + (j + 1) * pwu];
                    let a12 = a[i + 1 + (j + 2) * pwu];
                    let mut equals = 0;
                    if are_similar(mc, a10, a11) {
                        equals += 1;
                    }
                    if are_similar(mc, a21, a11) {
                        equals += 1;
                    }
                    if are_similar(mc, a01, a11) {
                        equals += 1;
                    }
                    if are_similar(mc, a12, a11) {
                        equals += 1;
                    }
                    out[o] = if equals >= 3 + (mc <= V1_6) as i32 { hill } else { a11 };
                } else {
                    out[o] = a11;
                }
            } else {
                out[o] = a11;
            }
        }
    }
}

/// `reduceID`
#[inline]
fn reduce_id(id: i32) -> i32 {
    if id >= 2 {
        2 + (id & 1)
    } else {
        id
    }
}

/// `mapRiver`：河流边界提取（一致区域写 -1，边界写 `river`）。
fn map_river(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let mc = l.mc;
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        for i in 0..w as usize {
            let mut v01 = pbuf[row1 + i];
            let mut v11 = pbuf[row1 + i + 1];
            let mut v21 = pbuf[row1 + i + 2];
            let mut v10 = pbuf[row0 + i + 1];
            let mut v12 = pbuf[row2 + i + 1];
            let o = i + j * w as usize;

            if mc >= V1_7 {
                v01 = reduce_id(v01);
                v11 = reduce_id(v11);
                v21 = reduce_id(v21);
                v10 = reduce_id(v10);
                v12 = reduce_id(v12);
            } else if v11 == 0 {
                out[o] = RIVER;
                continue;
            }

            out[o] = if v11 == v01 && v11 == v10 && v11 == v12 && v11 == v21 {
                -1
            } else {
                RIVER
            };
        }
    }
}

/// `mapSmooth`：四邻成对一致时随机取向。
fn map_smooth(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let ss = l.start_seed;
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        for i in 0..w as usize {
            let mut v11 = pbuf[row1 + i + 1];
            let v01 = pbuf[row1 + i];
            let v10 = pbuf[row0 + i + 1];

            if v11 != v01 || v11 != v10 {
                let v21 = pbuf[row1 + i + 2];
                let v12 = pbuf[row2 + i + 1];
                if v01 == v21 && v10 == v12 {
                    let cs = chunk_seed(ss, i as i32 + x, j as i32 + z);
                    v11 = if cs & (1 << 24) != 0 { v10 } else { v01 };
                } else {
                    if v01 == v21 {
                        v11 = v01;
                    }
                    if v10 == v12 {
                        v11 = v10;
                    }
                }
            }
            out[i + j * w as usize] = v11;
        }
    }
}

/// `mapSunflower`（`mapRareBiome`）：平原 1/57 变向日葵平原。
fn map_sunflower(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p.unwrap(), out, x, z, w, h);
    let ss = l.start_seed;

    for j in 0..h {
        for i in 0..w {
            let o = (i + j * w) as usize;
            if out[o] == PLAINS {
                let cs = chunk_seed(ss, i + x, j + z);
                if first_is_zero(cs, 57) {
                    out[o] = SUNFLOWER_PLAINS;
                }
            }
        }
    }
}

/// `replaceOcean`
#[allow(clippy::too_many_arguments)]
fn replace_ocean(out: &mut [i32], idx: usize, v10: i32, v21: i32, v01: i32, v12: i32, id: i32, replace: i32) {
    if is_oceanic(id) {
        return;
    }
    out[idx] = if is_oceanic(v10) || is_oceanic(v21) || is_oceanic(v01) || is_oceanic(v12) {
        replace
    } else {
        id
    };
}

/// `isAll4JFTO`：四格均为丛林类/森林/针叶林/海洋。
fn is_all4_jfto(mc: McVersion, a: i32, b: i32, c: i32, d: i32) -> bool {
    [a, b, c, d].iter().all(|&v| {
        get_category(mc, v) == JUNGLE || v == FOREST || v == TAIGA || is_oceanic(v)
    })
}

/// `isAny4Oceanic`
#[inline]
fn is_any4_oceanic(a: i32, b: i32, c: i32, d: i32) -> bool {
    is_oceanic(a) || is_oceanic(b) || is_oceanic(c) || is_oceanic(d)
}

/// `mapShore`：海岸/石岸/雪滩与丛林、恶地边界（1.7+ 分支）。
///
/// 1.1–1.6 只处理 mountains→mountain_edge 与 beach；1.0 及更早除蘑菇岸外直通。
fn map_shore(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let mc = l.mc;
    let (pbuf, pw) = parent_area(ls, l, x, z, w, h);
    let pwu = pw as usize;

    for j in 0..h as usize {
        let (row0, row1, row2) = (j * pwu, (j + 1) * pwu, (j + 2) * pwu);
        for i in 0..w as usize {
            let mut v11 = pbuf[row1 + i + 1];
            let v10 = pbuf[row0 + i + 1];
            let v21 = pbuf[row1 + i + 2];
            let v01 = pbuf[row1 + i];
            let v12 = pbuf[row2 + i + 1];
            let o = i + j * w as usize;

            if v11 == MUSHROOM_FIELDS {
                out[o] = if is_any4(OCEAN, v10, v21, v01, v12) {
                    MUSHROOM_FIELD_SHORE
                } else {
                    v11
                };
                continue;
            }
            if mc <= V1_0 {
                out[o] = v11;
                continue;
            }

            if mc <= V1_6 {
                if v11 == MOUNTAINS {
                    if v10 != MOUNTAINS || v21 != MOUNTAINS || v01 != MOUNTAINS || v12 != MOUNTAINS {
                        v11 = MOUNTAIN_EDGE;
                    }
                } else if v11 != OCEAN && v11 != RIVER && v11 != SWAMP
                    && is_any4(OCEAN, v10, v21, v01, v12)
                {
                    v11 = BEACH;
                }
                out[o] = v11;
            } else if get_category(mc, v11) == JUNGLE {
                if is_all4_jfto(mc, v10, v21, v01, v12) {
                    out[o] = if is_any4_oceanic(v10, v21, v01, v12) {
                        BEACH
                    } else {
                        v11
                    };
                } else {
                    out[o] = JUNGLE_EDGE;
                }
            } else if v11 == MOUNTAINS || v11 == WOODED_MOUNTAINS {
                replace_ocean(out, o, v10, v21, v01, v12, v11, STONE_SHORE);
            } else if is_snowy(v11) {
                replace_ocean(out, o, v10, v21, v01, v12, v11, SNOWY_BEACH);
            } else if v11 == BADLANDS || v11 == WOODED_BADLANDS_PLATEAU {
                if !is_any4_oceanic(v10, v21, v01, v12) {
                    out[o] = if is_mesa(v10) && is_mesa(v21) && is_mesa(v01) && is_mesa(v12) {
                        v11
                    } else {
                        DESERT
                    };
                } else {
                    out[o] = v11;
                }
            } else if v11 != OCEAN && v11 != DEEP_OCEAN && v11 != RIVER && v11 != SWAMP {
                out[o] = if is_any4_oceanic(v10, v21, v01, v12) {
                    BEACH
                } else {
                    v11
                };
            } else {
                out[o] = v11;
            }
        }
    }
}

/// `mapRiverMix`：把河流叠加到群系链上。
fn map_river_mix(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p.unwrap(), out, x, z, w, h); // 群系链
    let mut rbuf = vec![0i32; (w * h) as usize];
    get_map(ls, l.p2.unwrap(), &mut rbuf, x, z, w, h); // 河流链
    let mc = l.mc;

    for o in 0..(w * h) as usize {
        let mut v = out[o];
        if rbuf[o] == RIVER && v != OCEAN && (mc <= V1_6 || !is_oceanic(v)) {
            if v == SNOWY_TUNDRA {
                v = FROZEN_RIVER;
            } else if v == MUSHROOM_FIELDS || v == MUSHROOM_FIELD_SHORE {
                v = MUSHROOM_FIELD_SHORE;
            } else {
                v = RIVER;
            }
        }
        out[o] = v;
    }
}

/// `mapOceanTemp`（1.13+）：Perlin 温度噪声决定海洋变体。
///
/// 注意 C 把世界 `(x, z)` 传给噪声的 `(d1, d2)` 轴（d2 即噪声 "y" 轴）。
fn map_ocean_temp(ls: &LayerStack, _idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let rnd = &ls.ocean_rnd;
    for j in 0..h {
        for i in 0..w {
            let tmp = rnd.sample((i + x) as f64 / 8.0, (j + z) as f64 / 8.0, 0.0, 0.0, 0.0);
            out[(i + j * w) as usize] = if tmp > 0.4 {
                WARM_OCEAN
            } else if tmp > 0.2 {
                LUKEWARM_OCEAN
            } else if tmp < -0.4 {
                FROZEN_OCEAN
            } else if tmp < -0.2 {
                COLD_OCEAN
            } else {
                OCEAN
            };
        }
    }
}

/// `mapOceanMix`（1.13+）：陆地链与海洋温度链混合。
fn map_ocean_mix(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    get_map(ls, l.p2.unwrap(), out, x, z, w, h); // 海洋链

    // 最小陆地需求区域（仅 warm/frozen 海洋需要 8 格边界）
    let (mut lx0, mut lx1, mut lz0, mut lz1) = (0, w, 0, h);
    for j in 0..h {
        let jcentre = j - 8 > 0 && j + 9 < h;
        for i in 0..w {
            if jcentre && i - 8 > 0 && i + 9 < w {
                continue;
            }
            let oid = out[(i + j * w) as usize];
            if oid == WARM_OCEAN || oid == FROZEN_OCEAN {
                lx0 = lx0.min(i - 8);
                lx1 = lx1.max(i + 9);
                lz0 = lz0.min(j - 8);
                lz1 = lz1.max(j + 9);
            }
        }
    }

    let (lw, lh) = (lx1 - lx0, lz1 - lz0);
    let mut land = vec![0i32; (lw * lh) as usize];
    get_map(ls, l.p.unwrap(), &mut land, x + lx0, z + lz0, lw, lh);

    for j in 0..h {
        // C 的 `goto loop_x` 跳到内层循环体末尾（即继续下一列 i），
        // 因此标签必须挂在内层循环上，不能挂外层 j 循环。
        'cell: for i in 0..w {
            let land_id = land[((i - lx0) + (j - lz0) * lw) as usize];
            let mut ocean_id = out[(i + j * w) as usize];
            let o = (i + j * w) as usize;

            if !is_oceanic(land_id) {
                out[o] = land_id;
                continue;
            }

            let mut replace = 0;
            if ocean_id == WARM_OCEAN {
                replace = LUKEWARM_OCEAN;
            }
            if ocean_id == FROZEN_OCEAN {
                replace = COLD_OCEAN;
            }
            if replace != 0 {
                for ii in (-8..=8).step_by(4) {
                    for jj in (-8..=8).step_by(4) {
                        let id = land[((i + ii - lx0) + (j + jj - lz0) * lw) as usize];
                        if !is_oceanic(id) {
                            out[o] = replace;
                            continue 'cell;
                        }
                    }
                }
            }

            if land_id == DEEP_OCEAN {
                ocean_id = match ocean_id {
                    LUKEWARM_OCEAN => DEEP_LUKEWARM_OCEAN,
                    OCEAN => DEEP_OCEAN,
                    COLD_OCEAN => DEEP_COLD_OCEAN,
                    FROZEN_OCEAN => DEEP_FROZEN_OCEAN,
                    other => other,
                };
            }
            out[o] = ocean_id;
        }
    }
}

/// `mapOceanMixMod`（generator.c 的 `FORCE_OCEAN_VARIANTS` 自定义路径，
/// 1.13+）：陆地链与海洋温度链在同区域已生成完毕后的逐格混合。
///
/// 与 [`map_ocean_mix`] 不同，这里没有 warm/frozen 海洋的岸边降级扫描：
/// 陆地为海洋格时直接取海洋变体（陆地为 `deep_ocean` 时把浅海变体
/// 升级为对应深海）。`land` 与 `ocean` 须等长。
pub fn map_ocean_mix_mod(land: &[i32], ocean: &[i32]) -> Vec<i32> {
    assert_eq!(land.len(), ocean.len(), "mapOceanMixMod: 输入长度不一致");
    let mut out = vec![0i32; land.len()];
    for o in 0..land.len() {
        let land_id = land[o];
        if !is_oceanic(land_id) {
            out[o] = land_id;
            continue;
        }
        let mut ocean_id = ocean[o];
        if land_id == DEEP_OCEAN {
            ocean_id = match ocean_id {
                LUKEWARM_OCEAN => DEEP_LUKEWARM_OCEAN,
                OCEAN => DEEP_OCEAN,
                COLD_OCEAN => DEEP_COLD_OCEAN,
                FROZEN_OCEAN => DEEP_FROZEN_OCEAN,
                other => other,
            };
        }
        out[o] = ocean_id;
    }
    out
}

/// `mapVoronoi114`：1.14- 的 1:1 voronoi 缩放层。
fn map_voronoi114(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let x = x - 2;
    let z = z - 2;
    let px = x >> 2;
    let pz = z >> 2;
    let pw = ((x + w) >> 2) - px + 2;
    let ph = ((z + h) >> 2) - pz + 2;

    let mut src = vec![0i32; (pw * ph) as usize];
    get_map(ls, l.p.unwrap(), &mut src, px, pz, pw, ph);

    // 核心算法见 voronoi::map_voronoi_114_plane（末地 scale 1 路径共用）。
    // C 把结果写进 out 之后的暂存区再 memmove 回来；循环覆盖全部输出格，
    // 直接写 out 等价。注意传参 x/z 要加回 2（核心函数内部会再减 2）。
    super::voronoi::map_voronoi_114_plane(
        l.start_salt,
        l.start_seed,
        &src,
        out,
        x + 2,
        z + 2,
        w,
        h,
    );
}

/// `mapVoronoi`（1.15+）：SHA 播种的 voronoi 平面缩放层。
fn map_voronoi(ls: &LayerStack, idx: usize, out: &mut [i32], x: i32, z: i32, w: i32, h: i32) {
    let l = &ls.layers[idx];
    let x = x - 2;
    let z = z - 2;
    let px = x >> 2;
    let pz = z >> 2;
    let pw = ((x + w) >> 2) - px + 2;
    let ph = ((z + h) >> 2) - pz + 2;

    let mut src = vec![0i32; (pw * ph) as usize];
    get_map(ls, l.p.unwrap(), &mut src, px, pz, pw, ph);
    // C 怪癖：mapVoronoi 先把父层数据写进 out（pw 宽行主序的前 w*h 格），
    // 再 memmove 到暂存区；mapVoronoiPlane 不覆盖的边缘输出格（如逐点查询
    // 时 (x-4)/(-z-4) 落在源区外的角）会残留这些父层数据而非 0。
    // 这里逐位复刻该行为（`min` 对应 C 中 out 容量限制）。
    let n = ((w * h) as usize).min(src.len());
    out[..n].copy_from_slice(&src[..n]);
    // 旧版主世界无垂直噪声，y 恒为 0
    map_voronoi_plane(l.start_salt, out, &src, x, z, w, h, 0, px, pz, pw, ph);
}
