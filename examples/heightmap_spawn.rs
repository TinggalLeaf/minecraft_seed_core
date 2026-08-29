//! 地表高度近似 + 精确出生点（get_spawn）与估计出生点（estimate_spawn）对比。
//!
//! 运行：cargo run --example heightmap_spawn

use minecraft_seed_core::generator::Generator;
use minecraft_seed_core::structure::{estimate_spawn, get_spawn};
use minecraft_seed_core::{Dimension, McVersion};

fn main() {
    let mc = McVersion::V1_20;
    for seed in [12345u64, 0, 1] {
        let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);
        let est = estimate_spawn(&g);
        let exact = get_spawn(&g);
        println!(
            "seed {:>6}: estimate ({:>5},{:>5}) → 精确 ({:>5},{:>5})",
            seed as i64, est.x, est.z, exact.x, exact.z
        );
    }

    println!("\n=== 近似地表高度图（1:4 比例，8x8，种子 12345）===");
    let g = Generator::new(mc).with_seed(Dimension::Overworld, 12345);
    let h = g.map_approx_height(-4, -4, 8, 8);
    for row in 0..8 {
        let line: Vec<String> = (0..8)
            .map(|col| format!("{:>4.0}", h.y[row * 8 + col]))
            .collect();
        println!("{}", line.join(" "));
    }
}
