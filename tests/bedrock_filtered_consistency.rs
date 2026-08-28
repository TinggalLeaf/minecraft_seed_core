//! Bedrock 群系过滤版结构定位（`be_get_filtered_structures_in_regions`，wasm func21）
//! 与网站真实 bedrock.wasm 的端到端对拍。
//!
//! golden 数据由 `reference/site/dump_bedrock_golden.mjs` 生成（Node 直接实例化
//! 网站 wasm 的导出 `p`），见 [`bedrock_filtered_golden_data`] 头注释。
//!
//! 除逐用例精确比对外，还验证一致性不变量：**过滤版是非过滤版的子集**
//! （func21 本身就是 func14 候选 + 群系过滤）。
//!
//! 注意：mcseedmap.com 自身未启用过滤版（其 bedrock-worker.js 注释说明地图
//! 底图复用 Java 引擎），此处为算法完整性对拍。

#[path = "bedrock_filtered_golden_data.rs"]
mod bedrock_filtered_golden_data;

#[path = "bedrock_golden_data.rs"]
mod bedrock_golden_data;

use bedrock_filtered_golden_data::{FILTERED_CASES, STRUCT_NAMES};
use minecraft_seed_core::bedrock::{
    structures_in_regions, structures_in_regions_filtered, BeStructureType, BedrockVersion,
};

fn map_stype(stype: i32) -> BeStructureType {
    BeStructureType::ALL
        .iter()
        .copied()
        .find(|t| t.stype() == stype)
        .unwrap_or_else(|| panic!("未知 stype {stype}"))
}

#[test]
fn filtered_structures_match_wasm() {
    for (ci, case) in FILTERED_CASES.iter().enumerate() {
        let version = BedrockVersion::from_mc(case.mc).unwrap();
        for (i, &(name, stype)) in STRUCT_NAMES.iter().enumerate() {
            let got = structures_in_regions_filtered(version, map_stype(stype), case.seed, 2);
            assert_eq!(
                got,
                case.structures[i],
                "过滤版不一致 mc={} {} seed={}（{} 项 vs {} 项）",
                case.mc,
                name,
                case.seed,
                got.len(),
                case.structures[i].len()
            );
        }
        // 与同一用例的非过滤版 golden 对齐（bedrock_golden_data.rs 的 CASES 同序）
        assert_eq!(bedrock_golden_data::CASES[ci].mc, case.mc);
        assert_eq!(bedrock_golden_data::CASES[ci].seed, case.seed);
    }
}

#[test]
fn filtered_is_subset_of_unfiltered() {
    // 抽样若干用例（全量已由上面的精确对拍覆盖，这里验证不变量本身）
    for case in FILTERED_CASES.iter().step_by(7) {
        let version = BedrockVersion::from_mc(case.mc).unwrap();
        for &(_, stype) in STRUCT_NAMES.iter() {
            let st = map_stype(stype);
            let all = structures_in_regions(version, st, case.seed, 2);
            let filtered = structures_in_regions_filtered(version, st, case.seed, 2);
            for p in &filtered {
                assert!(all.contains(p), "过滤结果 {:?} 不在非过滤候选中", p);
            }
        }
    }
}
