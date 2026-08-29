//! 1.18+ 气候多噪声采样（BiomeNoise）：六点气候参数与群系判定。
//!
//! 运行：cargo run --example climate_noise

use minecraft_seed_core::noise::biome_noise::BiomeNoise;
use minecraft_seed_core::{Dimension, Generator, McVersion};

fn main() {
    let mc = McVersion::V1_20;
    let seed = 12345u64;

    let mut bn = BiomeNoise::new(mc);
    bn.set_biome_seed(seed, false);

    println!("种子 {seed} 的气候参数（scale-4 采样，y=80）：");
    println!("{:>10} {:>10} {:>10} {:>10} {:>10} {:>10}", "x", "temp", "humid", "cont", "erosion", "weird");
    for x in (-4..=4).step_by(2) {
        // sample_np 输出 [temp, humid, cont, erosion, depth, weird]（i64 定点）
        let np = bn.sample_np(x, 80, 0, 0);
        let f = |v: i64| v as f64 / 10000.0;
        println!(
            "{:>10} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>10.4}",
            x * 4,
            f(np[0]),
            f(np[1]),
            f(np[2]),
            f(np[3]),
            f(np[5])
        );
    }

    println!("\n对应群系（Generator 对拍）：");
    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);
    for x in (-4..=4).step_by(2) {
        println!("  ({:>5}, 0) → {:?}", x * 4, g.get_biome(x, 80, 0));
    }
}
