//! 综合种子信息示例：出生点估计、前几座要塞、出生点附近的史莱姆区块。
//!
//! 运行：`cargo run --example seed_info`
use minecraft_seed_core::generator::Generator;
use minecraft_seed_core::structure::{estimate_spawn, is_slime_chunk, StrongholdIter};
use minecraft_seed_core::{Dimension, McVersion};

fn main() {
    let seed: u64 = 12345;
    let mc = McVersion::V1_20;
    println!("seed={seed}  mc={} ({mc:?})", mc.name());

    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);

    // ---- 出生点 ----
    // estimate_spawn 是近似值（mcseedmap 显示的即为该估计）：
    // 1.7–1.17 在 ±256 方块内找可行群系；1.18+ 做气候适应度搜索。
    let spawn = estimate_spawn(&g);
    println!("spawn ~= ({}, {})", spawn.x, spawn.z);
    // get_biome 的坐标是 1:4 比例（方块坐标 / 4）
    let biome = g.get_biome(spawn.x >> 2, 319 >> 2, spawn.z >> 2);
    println!("spawn biome: {biome:?} (id={})", biome as i32);

    // ---- 要塞 ----
    // 1.9+ 共 128 座（8 环带），1.8 及以前 3 座；这里打印前 3 座的精确位置。
    // 1.7–1.19.2 必须传主世界生成器做群系检查；1.19.3+ 可传 None 只取近似位置。
    let mut iter = StrongholdIter::new(mc, seed);
    for _ in 0..3 {
        iter.next(Some(&g));
        println!("stronghold #{}: ({}, {})", iter.index, iter.pos.x, iter.pos.z);
    }

    // ---- 史莱姆区块 ----
    // is_slime_chunk 只依赖种子与区块坐标，不需要生成器（Java 版规则）。
    let (cx, cz) = (spawn.x >> 4, spawn.z >> 4);
    println!("slime chunks within 4 chunks of spawn ({cx}, {cz}):");
    let mut n = 0;
    for z in cz - 4..=cz + 4 {
        for x in cx - 4..=cx + 4 {
            if is_slime_chunk(seed, x, z) {
                println!("  chunk ({x:3},{z:3}) -> blocks ({:5},{:5})", x * 16, z * 16);
                n += 1;
            }
        }
    }
    println!("total: {n} / 81 chunks");
}
