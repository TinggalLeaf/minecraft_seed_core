//! Bedrock 版：结构散布（含群系过滤版）、出生点、要塞。
//!
//! 运行：cargo run --example bedrock_demo

use minecraft_seed_core::bedrock::{
    BedrockVersion, BeStructureType, find_structures, get_config, get_spawn, get_strongholds,
    structures_in_regions, structures_in_regions_filtered,
};

fn main() {
    let seed = 12345i64;
    let v = BedrockVersion::V1_21_0;

    println!("=== Bedrock {} seed={} ===", v.name(), seed);
    let [sx, sz] = get_spawn(seed);
    println!("出生点: ({}, {})", sx, sz);
    println!("要塞: {:?}", get_strongholds(seed));

    println!("\n=== 结构配置（1.20.0 vs 1.18.0 分界）===");
    for v in [BedrockVersion::V1_17_40, BedrockVersion::V1_18_0, BedrockVersion::V1_21_0] {
        let c = get_config(v, BeStructureType::Village);
        println!(
            "{} 村庄: spacing={} separation={} salt={}",
            v.name(), c.spacing, c.separation, c.salt
        );
    }

    println!("\n=== 原点 ±4 region 内的村庄（非过滤 vs 群系过滤）===");
    let all = structures_in_regions(v, BeStructureType::Village, seed, 4);
    let filtered = structures_in_regions_filtered(v, BeStructureType::Village, seed, 4);
    println!("非过滤: {} 个候选", all.len());
    println!("过滤后: {} 个（前 5: {:?}）", filtered.len(), &filtered[..filtered.len().min(5)]);

    println!("\n=== 以 (1000, -2000) 为中心查找海底神殿 ===");
    let m = find_structures(v, BeStructureType::OceanMonument, seed, 1000, -2000, 2);
    println!("{:?}", m);
}
