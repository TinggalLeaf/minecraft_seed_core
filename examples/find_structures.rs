//! 结构查找示例：给定种子与版本，扫描指定范围内的村庄候选位置，
//! 并用群系生成器验证可行性。
//!
//! 运行：`cargo run --example find_structures`
use minecraft_seed_core::generator::Generator;
use minecraft_seed_core::structure::{
    get_config, get_structure_pos, get_variant, is_viable_structure_pos,
};
use minecraft_seed_core::{Dimension, McVersion, StructureType};

fn main() {
    let seed: u64 = 12345;
    let mc = McVersion::V1_20;
    let stype = StructureType::Village;

    // 1) 取该版本下的结构配置（版本不支持此结构时返回 None）
    let config = get_config(stype, mc).expect("该版本不支持村庄");
    println!(
        "seed={seed}  mc={} ({mc:?})  structure={stype:?}",
        mc.name()
    );
    println!(
        "config: region_size={} chunks, chunk_range={} chunks, salt={}",
        config.region_size, config.chunk_range, config.salt
    );

    // 2) 结构可行性检查需要主世界群系生成器
    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);

    // 3) 扫描以原点为中心 ±4 个 region
    //    （region 边长 = region_size 区块 = region_size*16 方块）
    const REG_R: i32 = 4;
    let (mut candidates, mut viable) = (0, 0);
    for rz in -REG_R..=REG_R {
        for rx in -REG_R..=REG_R {
            // 候选位置：只取决于结构类型、region 坐标与种子低 48 位；
            // 有些 region 里无论群系如何都不会生成（稀有度判定），返回 None
            let Some(pos) = get_structure_pos(stype, mc, seed, rx, rz) else {
                continue;
            };
            candidates += 1;

            // 群系可行性检查（1.18+ 村庄会返回可行的群系变体 ID）
            let v = is_viable_structure_pos(stype, &g, pos.x, pos.z, 0);
            if v == 0 {
                println!("  blocked region=({rx:2},{rz:2}) pos=({:6},{:6})", pos.x, pos.z);
                continue;
            }
            viable += 1;
            // 变体信息：朝向、起始部件、是否僵尸村庄等
            let sv = get_variant(stype, mc, seed, pos.x, pos.z, v).expect("viable 村庄应有变体");
            println!(
                "  VIABLE  region=({rx:2},{rz:2}) pos=({:6},{:6}) biome_id={v} \
                 rotation={} abandoned={} bbox={}x{}x{}",
                pos.x, pos.z, sv.rotation, sv.abandoned, sv.sx, sv.sy, sv.sz
            );
        }
    }
    println!("regions={}x{} candidates={candidates} viable={viable}", 2 * REG_R + 1, 2 * REG_R + 1);
}
