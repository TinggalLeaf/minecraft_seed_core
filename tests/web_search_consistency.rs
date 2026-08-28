//! 种子搜索 API 与 mcseedmap.com api.wasm 的端到端对拍。
//!
//! 数据源：`tests/web_search_golden_data.rs`（由 reference/site 探针从网站
//! api.wasm 的 find_biomes/find_structures/find_biomes_with_structure dump）。

#[path = "web_search_golden_data.rs"]
mod web_search_golden_data;

use minecraft_seed_core::search::find_biomes;
use minecraft_seed_core::{Dimension, McVersion};

fn map_mc(mc: i32) -> McVersion {
    match mc {
        10 => McVersion::V1_7,
        15 => McVersion::V1_12,
        16 => McVersion::V1_13,
        17 => McVersion::V1_14,
        20 => McVersion::V1_16,
        21 => McVersion::V1_17,
        22 => McVersion::V1_18,
        24 => McVersion::V1_19,
        25 => McVersion::V1_20,
        28 => McVersion::V1_21,
        _ => panic!("未知 mc {mc}"),
    }
}

fn map_dim(dim: i32) -> Dimension {
    match dim {
        0 => Dimension::Overworld,
        -1 => Dimension::Nether,
        1 => Dimension::End,
        _ => panic!(),
    }
}

#[test]
fn find_biomes_matches_website() {
    for c in web_search_golden_data::FIND_BIOMES_CASES {
        if c.expect < 0 {
            continue; // 超时案例不参与（搜索空间过大）
        }
        let got = find_biomes(
            map_mc(c.mc),
            map_dim(c.dim),
            c.ids,
            c.x,
            c.z,
            c.w,
            c.h,
            c.y_height,
            c.start,
        );
        assert_eq!(
            got, c.expect,
            "find_biomes 不一致 mc={} ids={:?} area=({},{},{}x{}) start={}",
            c.mc, c.ids, c.x, c.z, c.w, c.h, c.start
        );
    }
}

#[path = "find_struct_golden_data.rs"]
mod find_struct_golden_data;

use minecraft_seed_core::structure::StructureType;
use minecraft_seed_core::search::find_structures;

/// 网站结构类型编号 → cubiomes StructureType（wasm 的 stype 即 cubiomes 枚举序）。
fn map_stype(stype: i32) -> StructureType {
    match stype {
        5 => StructureType::Village,
        1 => StructureType::DesertPyramid,
        8 => StructureType::Monument,
        14 => StructureType::Treasure,
        9 => StructureType::Mansion,
        13 => StructureType::Outpost,
        _ => panic!("未映射 stype {stype}"),
    }
}

#[test]
fn find_structures_matches_website() {
    for c in find_struct_golden_data::FIND_STRUCT_CASES {
        let got = find_structures(
            map_mc(c.mc),
            map_dim(c.dim),
            map_stype(c.stype),
            c.x,
            c.z,
            c.range,
            c.start,
        );
        assert_eq!(
            got, c.expect,
            "find_structures 不一致 mc={} stype={} center=({},{}) range={} start={}",
            c.mc, c.stype, c.x, c.z, c.range, c.start
        );
    }
}

#[path = "find_wb_golden_data.rs"]
mod find_wb_golden_data;

use minecraft_seed_core::search::find_biomes_with_structure;

#[test]
fn find_biomes_with_structure_matches_website() {
    for c in find_wb_golden_data::FIND_WB_CASES {
        let got = find_biomes_with_structure(
            map_mc(c.mc),
            map_dim(c.dim),
            map_stype(c.stype),
            c.biomes,
            c.x,
            c.z,
            c.range,
            c.y_height,
            c.start,
        );
        assert_eq!(
            got, c.expect,
            "find_biomes_with_structure 不一致 mc={} stype={} biomes={:?} center=({},{}) range={} start={}",
            c.mc, c.stype, c.biomes, c.x, c.z, c.range, c.start
        );
    }
}
