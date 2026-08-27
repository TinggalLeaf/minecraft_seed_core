//! 手动性能冒烟：1.7–1.17 分层群系源的区域生成耗时。
use minecraft_seed_core::{Dimension, Generator, McVersion};
use minecraft_seed_core::generator::Range;
use std::time::Instant;

fn main() {
    for mc in [McVersion::V1_7, McVersion::V1_12, McVersion::V1_13, McVersion::V1_17] {
        let g = Generator::new(mc).with_seed(Dimension::Overworld, 12345);
        // 预热
        let _ = g.gen_biomes(Range::new(4, 0, 0, 64, 64));
        let t = Instant::now();
        for _ in 0..100 {
            let _ = g.gen_biomes(Range::new(4, 0, 0, 64, 64));
        }
        let s4 = t.elapsed() / 100;
        let t = Instant::now();
        for _ in 0..10 {
            let _ = g.gen_biomes(Range::new(1, -256, -256, 512, 512));
        }
        let s1 = t.elapsed() / 10;
        println!("{:>6?}: 64x64@scale4 = {:?}, 512x512@scale1 = {:?}", mc, s4, s1);
    }
}
