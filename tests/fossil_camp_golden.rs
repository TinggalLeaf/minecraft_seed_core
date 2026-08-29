//! 化石（Fossil）与废弃营地（AbandonedCamp）的 golden 对拍。
//!
//! Golden 数据由 `reference/site/dump_fossil_golden.mjs` 生成——该脚本
//! 逐位复刻 mcseedmap 前端 `chunk-874.js` 的 stype=26/27 散布分支
//! （ef=xoroshiro128+、en=Java LCG、ed=chunk 种子派生）。

mod fossil_camp_golden_data;

use fossil_camp_golden_data::{CAMP_CASES, FOSSIL_CASES};
use minecraft_seed_core::structure::{
    get_config, get_structure_pos, scan_fossils, StructureType,
};

#[test]
fn fossil_positions_match_website_js() {
    for (mc, seed, expected) in FOSSIL_CASES {
        let mut got: Vec<(i32, i32)> = scan_fossils(*mc, *seed, 0, 0, 15, 15)
            .into_iter()
            .map(|p| (p.x, p.z))
            .collect();
        // 网站 JS 按 chunk 行优先序输出，本库 scan_fossils 同序
        let exp: Vec<(i32, i32)> = expected.to_vec();
        got.dedup();
        assert_eq!(
            got.len(),
            exp.len(),
            "fossil count mismatch: mc={mc:?} seed={seed}"
        );
        assert_eq!(got, exp, "fossil positions mismatch: mc={mc:?} seed={seed}");
    }
}

#[test]
fn abandoned_camp_positions_match_website_js() {
    let mc = minecraft_seed_core::McVersion::V1_21;
    let stype = StructureType::AbandonedCamp;
    assert!(get_config(stype, mc).is_some());
    for (seed, expected) in CAMP_CASES {
        let mut got = Vec::new();
        for rz in -2..=2 {
            for rx in -2..=2 {
                let p = get_structure_pos(stype, mc, *seed, rx, rz)
                    .expect("camp 应总是生成");
                got.push((p.x, p.z));
            }
        }
        // 网站 JS 输出顺序：外层 e(rx) 内层 t(rz)；本库外层 rz 内层 rx。
        // 位置集合无关顺序，排序后比对。
        got.sort();
        let mut exp: Vec<(i32, i32)> = expected.to_vec();
        exp.sort();
        assert_eq!(got, exp, "camp positions mismatch: seed={seed}");
    }
}

#[test]
fn abandoned_camp_config_values() {
    let c = get_config(StructureType::AbandonedCamp, minecraft_seed_core::McVersion::V1_21)
        .unwrap();
    assert_eq!(c.salt, 91231127);
    assert_eq!(c.region_size, 34);
    assert_eq!(c.chunk_range, 26);
    assert_eq!(c.dim, 0);
}

#[test]
fn fossil_has_no_region_config() {
    // 化石是逐区块散布，不走 StructureConfig
    assert!(get_config(StructureType::Fossil, minecraft_seed_core::McVersion::V1_21).is_none());
}
