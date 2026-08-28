//! 与 mcseedmap.com 真实后端的端到端一致性对拍。
//!
//! golden 数据来自网站实际的 WASM 引擎（`reference/site/api.wasm`，
//! v=26.3-terrain-3，cubiomes 编译产物），由 `reference/site/dump_golden.mjs`
//! 在 Node 中直接实例化导出，见 [`web_golden_data`] 头注释。
//!
//! 已核实的语义（详见 `docs/INTEGRATION.md`）：
//! - `find_spawn` = cubiomes `getSpawn`（含地形修正），本库 `get_spawn`
//!   与其逐位一致，直接精确比较。
//! - `generate_area` 的 x/z 是 1:4 群系比例坐标（不是方块坐标）；
//!   yHeight=320 时 y 传 80（quart）或 320 结果一致（均在地表之上）。
//! - `get_structure_in_regions(range=8)` 覆盖以原点为中心的
//!   [-8, 8) × [-8, 8) region 网格。

#[path = "web_golden_data.rs"]
mod web_golden_data;

use std::collections::BTreeSet;

use minecraft_seed_core::generator::{Generator, Range};
use minecraft_seed_core::structure::{
    get_config, get_spawn, get_structure_pos, is_viable_structure_pos, StrongholdIter,
    StructureType,
};
use minecraft_seed_core::{Dimension, McVersion};
use web_golden_data::WebCase;

/// 网站 cubiomes MCVersion 整数 → 本库版本枚举。
fn map_mc(mc: i32) -> McVersion {
    match mc {
        10 => McVersion::V1_7,     // 1.7
        15 => McVersion::V1_12,    // 1.12
        16 => McVersion::V1_13,    // 1.13
        17 => McVersion::V1_14,    // 1.14
        20 => McVersion::V1_16,    // 1.16.5
        21 => McVersion::V1_17,    // 1.17
        22 => McVersion::V1_18,    // 1.18
        24 => McVersion::V1_19,    // 1.19.4
        25 => McVersion::V1_20,    // 1.20.6
        28 => McVersion::V1_21,    // 1.21.4
        _ => panic!("未知网站 mc 版本号 {mc}"),
    }
}

/// `structures` 数组下标对应的结构类型（cubiomes StructureType 枚举序，
/// 见 dump_golden.mjs 的 structTypes 表）。
const WEB_STRUCT_TYPES: [(&str, StructureType); 11] = [
    ("village", StructureType::Village),                  // 5
    ("desert_pyramid", StructureType::DesertPyramid),     // 1
    ("jungle_temple", StructureType::JungleTemple),       // 2
    ("swamp_hut", StructureType::SwampHut),               // 3
    ("igloo", StructureType::Igloo),                      // 4
    ("monument", StructureType::Monument),                // 8
    ("mansion", StructureType::Mansion),                  // 9
    ("outpost", StructureType::Outpost),                  // 10
    ("ruined_portal", StructureType::RuinedPortal),       // 11
    ("treasure", StructureType::Treasure),                // 14
    ("mineshaft", StructureType::Mineshaft),              // 15
];

fn case_gen(c: &WebCase) -> Generator {
    Generator::new(map_mc(c.mc)).with_seed(Dimension::Overworld, c.seed as u64)
}

/// 出生点：网站 `find_spawn` = cubiomes `getSpawn`，本库 `get_spawn`
/// 与其逐函数等价（另见 `bundle_b_golden.rs` 的 C 参考对拍），50 个用例
/// 全部精确相等断言。
#[test]
fn web_spawn_matches_get_spawn_exactly() {
    for c in web_golden_data::CASES {
        let g = case_gen(c);
        let pos = get_spawn(&g);
        let (wx, wz) = (c.spawn[0], c.spawn[1]);
        assert_eq!(
            (pos.x, pos.z),
            (wx, wz),
            "{} seed={}: get_spawn 与网站 find_spawn 不一致",
            c.version,
            c.seed
        );
    }
}

/// 要塞：前 10 座逐一精确相等（1.8 及以前只有 3 座，网站以 -1 填充）。
#[test]
fn web_strongholds_exact() {
    for c in web_golden_data::CASES {
        let mc = map_mc(c.mc);
        let g = case_gen(c);
        let mut sh = StrongholdIter::new(mc, c.seed as u64);
        let max_count = if mc >= McVersion::V1_9 { 128 } else { 3 };
        let mut ours = Vec::new();
        for _ in 0..10 {
            if sh.index >= max_count {
                break;
            }
            sh.next(Some(&g));
            ours.push((sh.pos.x, sh.pos.z));
        }
        let mut web = Vec::new();
        for i in 0..10 {
            let (x, z) = (c.strongholds[2 * i], c.strongholds[2 * i + 1]);
            if x != -1 || z != -1 {
                web.push((x, z));
            }
        }
        assert_eq!(ours, web, "{} seed={} 要塞列表不一致", c.version, c.seed);
    }
}

/// 群系区域：64x64 @ scale 4、起点 (-128,-128)，4096 个 id 逐一精确相等。
///
/// 网站 generate_area 的 x/z 是 1:4 群系比例坐标（与 cubiomes Range 一致，
/// 不需换算）；y 在 scale 4 下为 1:4 垂直单位，yHeight=320 → 80。实测
/// y=80 与 y=320 对该区域输出完全相同（均位于地表之上，无洞穴群系）。
#[test]
fn web_biome_area_exact() {
    for c in web_golden_data::CASES {
        let [x, z, w, h, _dim, y_height, scale] = c.area;
        assert_eq!((scale, w, h), (4, 64, 64), "golden 参数假设已变");
        let g = case_gen(c);
        let out = g.gen_biomes(Range::new(scale, x, z, w, h).with_y(y_height / 4, 1));
        assert_eq!(out.len(), c.biome_ids.len());
        for (i, (a, b)) in out.iter().zip(c.biome_ids).enumerate() {
            assert_eq!(
                *a as i32, *b,
                "{} seed={} 群系区域第 {} 个采样点不一致",
                c.version, c.seed, i
            );
        }
    }
}

/// 结构：对 [-8, 8)² region 网格逐 region 计算候选位置 + 群系可行性，
/// 与网站列表做集合相等比较（顺序由 region 遍历顺序决定，不保证一致）。
#[test]
fn web_structures_exact() {
    for c in web_golden_data::CASES {
        let mc = map_mc(c.mc);
        let g = case_gen(c);
        let seed = c.seed as u64;
        for (i, (name, stype)) in WEB_STRUCT_TYPES.iter().enumerate() {
            let web: BTreeSet<(i32, i32)> =
                c.structures[i].iter().map(|p| (p[0], p[1])).collect();
            let mut ours = BTreeSet::new();
            if get_config(*stype, mc).is_some() {
                for rx in -8..8 {
                    for rz in -8..8 {
                        let Some(pos) = get_structure_pos(*stype, mc, seed, rx, rz) else {
                            continue;
                        };
                        if is_viable_structure_pos(*stype, &g, pos.x, pos.z, 0) != 0 {
                            ours.insert((pos.x, pos.z));
                        }
                    }
                }
            }
            assert_eq!(
                ours,
                web,
                "{} seed={} 结构 {name} 集合不一致",
                c.version,
                c.seed
            );
        }
    }
}
