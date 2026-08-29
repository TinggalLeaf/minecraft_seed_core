//! 地形级结构可行性：沙漠神殿四角深度检查、末地城高度检查。
//!
//! 运行：cargo run --example terrain_viability

use minecraft_seed_core::structure::{
    StructureType, get_structure_pos, is_viable_structure_pos, is_viable_structure_terrain,
};
use minecraft_seed_core::{Dimension, Generator, McVersion};

#[allow(clippy::collapsible_if)]
fn main() {
    let mc = McVersion::V1_20;
    let seed = 2u64;
    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);

    println!("沙漠神殿（群系可行 + 地形可行）扫描 region [-16,16]²（seed 2）：");
    let mut n_biome = 0;
    let mut n_both = 0;
    for rx in -16..=16 {
        for rz in -16..=16 {
            if let Some(pos) = get_structure_pos(StructureType::DesertPyramid, mc, seed, rx, rz) {
                if is_viable_structure_pos(StructureType::DesertPyramid, &g, pos.x, pos.z, 0) != 0 {
                    n_biome += 1;
                    // 地形检查：四角的地表近似高度 depth 参数
                    if is_viable_structure_terrain(StructureType::DesertPyramid, &g, pos.x, pos.z)
                    {
                        n_both += 1;
                        println!("  ({:>6}, {:>6}) 群系+地形均可行", pos.x, pos.z);
                    }
                }
            }
        }
    }
    println!("群系可行 {} 处，其中地形也可行 {} 处", n_biome, n_both);
}
