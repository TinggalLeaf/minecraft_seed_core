//! 下界与末地群系生成 + 末地折跃门定位。
//!
//! 运行：cargo run --example dimensions

use minecraft_seed_core::generator::{Generator, Range};
use minecraft_seed_core::structure::{get_linked_gateway_pos, is_end_chunk_empty};
use minecraft_seed_core::{BiomeId, Dimension, McVersion};

fn main() {
    let seed = 12345u64;
    let mc = McVersion::V1_20;

    println!("=== 下界群系（1.16+ 多噪声）===");
    let g = Generator::new(mc).with_seed(Dimension::Nether, seed);
    let area = g.gen_biomes(Range::new(4, -4, -4, 8, 8));
    let mut uniq: Vec<BiomeId> = area.clone();
    uniq.sort_by_key(|b| *b as i32);
    uniq.dedup();
    println!("8x8 区域（scale 4）内的群系: {:?}", uniq);
    println!("(0, 64, 0) 群系: {:?}", g.get_biome(0, 16, 0)); // y 以 1:4 计

    println!("\n=== 末地群系（外岛）===");
    let g = Generator::new(mc).with_seed(Dimension::End, seed);
    let area = g.gen_biomes(Range::new(4, 1000 / 4, 1000 / 4, 8, 8));
    let mut uniq: Vec<BiomeId> = area.clone();
    uniq.sort_by_key(|b| *b as i32);
    uniq.dedup();
    println!("(1000,1000) 附近 8x8 区域群系: {:?}", uniq);

    println!("\n=== 末地折跃门（主岛 → 外岛链接点）===");
    let en = g.end_noise().expect("end noise");
    let sn = g.surface_noise();
    let src = minecraft_seed_core::structure::Pos { x: 1024, z: 0 };
    println!("区块 (64, 0) 为空: {}", is_end_chunk_empty(en, sn, seed, 64, 0));
    let dst = get_linked_gateway_pos(en, sn, seed, src);
    println!("从 ({},{}) 出发的链接折跃门落点: ({}, {})", src.x, src.z, dst.x, dst.z);
}
