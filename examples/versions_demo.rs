//! 版本遍历：同一种子在不同版本下的群系差异 + large biomes 世界类型。
//!
//! 运行：cargo run --example versions_demo

use minecraft_seed_core::generator::{Generator, Range};
use minecraft_seed_core::{BiomeId, Dimension, McVersion};

fn main() {
    let seed = 12345u64;
    println!("=== 同一种子 {seed} 在不同版本的 (0,0) 群系（y=80，scale-4）===");
    for &mc in McVersion::ALL {
        let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);
        println!("{:<10} → {:?}", mc.name(), g.get_biome(0, 80, 0));
    }

    println!("\n=== 默认 vs large biomes（1.12.2，16x16 区域的不同群系数）===");
    let area = Range::new(4, -8, -8, 16, 16);
    let g = Generator::new(McVersion::V1_12).with_seed(Dimension::Overworld, seed);
    let biomes = g.gen_biomes(area);
    let mut uniq: Vec<BiomeId> = biomes.to_vec();
    uniq.sort_by_key(|b| *b as i32);
    uniq.dedup();
    println!("默认世界类型: {} 种群系", uniq.len());

    let gl = Generator::new(McVersion::V1_12)
        .with_large_biomes(true)
        .with_seed(Dimension::Overworld, seed);
    let biomes_l = gl.gen_biomes(area);
    let mut uniq_l = biomes_l.to_vec();
    uniq_l.sort_by_key(|b| *b as i32);
    uniq_l.dedup();
    println!("large biomes: {} 种群系", uniq_l.len());
}
