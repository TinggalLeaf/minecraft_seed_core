//! 与 mcseedmap.com Bedrock 后端（`reference/site/bedrock.wasm`）的端到端一致性对拍。
//!
//! golden 数据由 `reference/site/dump_bedrock_golden.mjs` 在 Node 中直接实例化
//! 网站 wasm 导出函数生成，见 [`bedrock_golden_data`] 头注释。
//!
//! 已核实的语义：
//! - `spawn`/`strongholds` 只用种子低 32 位、与 mc 版本无关（wasm 忽略 mc 参数）；
//! - `structures` 为 `be_get_structures_in_regions(mc, stype, seed, range=2)`
//!   （以原点为中心 ±2 region 的网格，15 个非退化类型）；
//! - `find_cases` 为 `be_find_structures(mc, stype, seed, cx, cz, range<<9)`，
//!   覆盖负数中心坐标的 region 换算。

#[path = "bedrock_golden_data.rs"]
mod bedrock_golden_data;

use bedrock_golden_data::{CASES, CONFIG_CASES, FIND_CASES, MT_VECTORS, STRUCT_NAMES};
use minecraft_seed_core::bedrock::{
    self, find_structures, get_config_raw, get_spawn, get_strongholds, structures_in_regions,
    BeStructureType, BedrockVersion,
};

/// stype 整数 → 结构类型枚举（golden 只含网站定义的 20 种）。
fn map_stype(stype: i32) -> BeStructureType {
    BeStructureType::ALL
        .iter()
        .copied()
        .find(|t| t.stype() == stype)
        .unwrap_or_else(|| panic!("未知 stype {stype}"))
}

#[test]
fn mt19937_vectors_match() {
    for v in MT_VECTORS {
        assert_eq!(
            bedrock::mt::mt_outputs(v.seed as u32, 8).as_slice(),
            &v.seq,
            "MT19937 seed={} 不一致",
            v.seed
        );
    }
}

#[test]
fn structure_configs_match() {
    for c in CONFIG_CASES {
        let cfg = get_config_raw(c.mc, c.stype);
        assert_eq!(
            [cfg.spacing, cfg.separation, cfg.salt, cfg.mt_count],
            c.values,
            "配置不一致 mc={} stype={}",
            c.mc,
            c.stype
        );
    }
}

#[test]
fn spawn_matches() {
    for case in CASES {
        assert_eq!(
            get_spawn(case.seed),
            case.spawn,
            "spawn 不一致 {} seed={}",
            case.version,
            case.seed
        );
    }
}

#[test]
fn strongholds_match() {
    for case in CASES {
        assert_eq!(
            get_strongholds(case.seed),
            case.strongholds,
            "strongholds 不一致 {} seed={}",
            case.version,
            case.seed
        );
    }
}

#[test]
fn structures_in_regions_match() {
    for case in CASES {
        let version = BedrockVersion::from_mc(case.mc).unwrap();
        for (i, &(name, stype)) in STRUCT_NAMES.iter().enumerate() {
            let got = structures_in_regions(version, map_stype(stype), case.seed, 2);
            assert_eq!(
                got,
                case.structures[i],
                "结构不一致 {} {} seed={}（{} 项 vs {} 项）",
                case.version,
                name,
                case.seed,
                got.len(),
                case.structures[i].len()
            );
        }
    }
}

#[test]
fn find_structures_match() {
    for fc in FIND_CASES {
        let version = BedrockVersion::from_mc(fc.mc).unwrap();
        let got = find_structures(
            version,
            map_stype(fc.stype),
            fc.seed,
            fc.center[0],
            fc.center[1],
            2,
        );
        assert_eq!(
            got,
            fc.positions,
            "find 不一致 mc={} stype={} seed={} center={:?}（{} 项 vs {} 项）",
            fc.mc,
            fc.stype,
            fc.seed,
            fc.center,
            got.len(),
            fc.positions.len()
        );
    }
}
