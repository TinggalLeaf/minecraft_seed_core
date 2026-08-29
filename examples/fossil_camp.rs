//! 化石（Fossil）与废弃营地（AbandonedCamp）示例。
//!
//! - 化石：1.20+ 主世界逐区块散布（双 salt 各 1/64），vanilla 限沙漠/
//!   沼泽/红树林沼泽群系；
//! - 废弃营地：网站标注 26.3-s1+，算法层 1.21.4+ 可用的 scattered
//!   feature（salt=91231127、region 34、range 26）。
//!
//! 运行：`cargo run --example fossil_camp`
use minecraft_seed_core::generator::Generator;
use minecraft_seed_core::structure::{
    get_config, get_fossil_positions, get_structure_pos, is_viable_feature_biome,
    is_viable_structure_pos, scan_fossils,
};
use minecraft_seed_core::{BiomeId, Dimension, McVersion, StructureType};

const SEED: u64 = 1085393142614036966;

fn main() {
    // ---- 化石：版本门控 ----
    println!("=== 化石版本门控 ===");
    for mc in [McVersion::V1_19_2, McVersion::V1_20, McVersion::V1_21] {
        let n = scan_fossils(mc, SEED, 0, 0, 63, 63).len();
        println!("  {}: 64×64 区块内 {} 处化石候选", mc.name(), n);
    }

    // ---- 化石：位置 + 群系过滤 ----
    let mc = McVersion::V1_21;
    let g = Generator::new(mc).with_seed(Dimension::Overworld, SEED);
    println!("\n=== 化石位置（出生点附近 ±32 区块，含群系过滤） ===");
    let (mut total, mut viable) = (0, 0);
    for cz in -32..=32 {
        for cx in -32..=32 {
            for pos in get_fossil_positions(mc, SEED, cx, cz) {
                total += 1;
                // vanilla 化石只生成于沙漠/沼泽/红树林沼泽
                let biome = g.get_biome(pos.x >> 2, 320 >> 2, pos.z >> 2);
                if is_viable_feature_biome(mc, StructureType::Fossil, biome as i32) {
                    viable += 1;
                    println!("  ({:>5}, {:>5})  {:?}", pos.x, pos.z, biome);
                }
            }
        }
    }
    println!("候选 {total} 处，落在合法群系的 {viable} 处");

    // ---- 废弃营地 ----
    println!("\n=== 废弃营地（AbandonedCamp） ===");
    let conf = get_config(StructureType::AbandonedCamp, mc).expect("1.21.4+ 可用");
    println!(
        "config: salt={} region={} chunks range={} chunks",
        conf.salt, conf.region_size, conf.chunk_range
    );
    let mut n = 0;
    for rz in -3..=3 {
        for rx in -3..=3 {
            if let Some(pos) = get_structure_pos(StructureType::AbandonedCamp, mc, SEED, rx, rz)
            {
                // 网站前端无群系过滤，is_viable 恒可行
                if is_viable_structure_pos(StructureType::AbandonedCamp, &g, pos.x, pos.z, 0) != 0
                {
                    n += 1;
                    if n <= 8 {
                        let d = ((pos.x as f64).powi(2) + (pos.z as f64).powi(2)).sqrt();
                        println!("  camp @ ({:>6}, {:>6})  距原点 {d:.0} 格", pos.x, pos.z);
                    }
                }
            }
        }
    }
    println!("±3 region 内共 {n} 处");

    // 旧版本不可用
    assert!(get_config(StructureType::AbandonedCamp, McVersion::V1_20).is_none());
    println!("\n1.20.6 及更早版本不支持废弃营地（get_config 返回 None）");

    // 群系 id 检查：沙漠群系中的化石判定
    let desert = BiomeId::Desert;
    assert!(is_viable_feature_biome(mc, StructureType::Fossil, desert as i32));
    let forest = BiomeId::Forest;
    assert!(!is_viable_feature_biome(mc, StructureType::Fossil, forest as i32));
    println!("群系过滤：desert ✓ / forest ✗");
}
