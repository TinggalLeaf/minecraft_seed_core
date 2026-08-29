//! loot 模块性能实测：查找、解析缓存、单线程 vs 多线程蒙特卡洛。
//!
//! 运行：`cargo run --release --example perf_loot`
use minecraft_seed_core::loot::{self, LootVersion};
use std::time::Instant;

fn main() {
    let v = LootVersion::V1_20_1;
    let ids: Vec<&str> = v.tables().iter().map(|(id, _)| *id).collect();

    // ---- 1. get_raw 查找（二分） ----
    let t = Instant::now();
    let mut n = 0usize;
    for _ in 0..100 {
        for id in &ids {
            n += v.get_raw(id).unwrap().len();
        }
    }
    let d = t.elapsed();
    println!(
        "get_raw ×{} 次：{:?}（{:.0} 次/秒，总字节 {n}）",
        ids.len() * 100,
        d,
        ids.len() as f64 * 100.0 / d.as_secs_f64()
    );

    // ---- 2. get（每次解析）vs get_cached（缓存） ----
    let t = Instant::now();
    for _ in 0..1000 {
        let _ = v.get("minecraft:chests/ruined_portal").unwrap();
    }
    let d_parse = t.elapsed();
    let t = Instant::now();
    for _ in 0..1000 {
        let _ = v.get_cached("minecraft:chests/ruined_portal").unwrap();
    }
    let d_cached = t.elapsed();
    println!(
        "ruined_portal ×1000：每次解析 {:?}，缓存 {:?}（{:.0}× 提速）",
        d_parse,
        d_cached,
        d_parse.as_secs_f64() / d_cached.as_secs_f64()
    );

    // ---- 3. 蒙特卡洛：单线程 vs 多线程 ----
    let table = v.get_cached("minecraft:chests/desert_pyramid").unwrap();
    const SAMPLES: usize = 100_000;
    let t = Instant::now();
    let single = loot::simulate(&table, 12345, SAMPLES, 0.0);
    let d1 = t.elapsed();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let t = Instant::now();
    let par = loot::simulate_par(&table, 12345, SAMPLES, 0.0, threads);
    let d2 = t.elapsed();
    assert_eq!(single, par, "并行结果必须与单线程一致");
    println!(
        "simulate ×{SAMPLES}：单线程 {:?}，{threads} 线程 {:?}（{:.1}× 提速）",
        d1,
        d2,
        d1.as_secs_f64() / d2.as_secs_f64()
    );

    // ---- 4. 并行结构扫描 ----
    use minecraft_seed_core::structure::{get_structure_pos, is_viable_structure_pos};
    use minecraft_seed_core::{Dimension, Generator, McVersion, StructureType};
    let mc = McVersion::V1_21;
    let g = Generator::new(mc).with_seed(Dimension::Overworld, 12345);
    let t = Instant::now();
    let mut single_n = 0;
    for rz in -32..=32 {
        for rx in -32..=32 {
            if let Some(p) = get_structure_pos(StructureType::Village, mc, 12345, rx, rz) {
                if is_viable_structure_pos(StructureType::Village, &g, p.x, p.z, 0) != 0 {
                    single_n += 1;
                }
            }
        }
    }
    let d1 = t.elapsed();
    let t = Instant::now();
    let par = minecraft_seed_core::structure::find_structures_par(
        StructureType::Village,
        mc,
        Dimension::Overworld,
        12345,
        -32..=32,
        -32..=32,
        threads,
    );
    let d2 = t.elapsed();
    assert_eq!(single_n, par.len(), "并行扫描结果必须与单线程一致");
    println!(
        "village 扫描 65×65 region：单线程 {:?}，{threads} 线程 {:?}（{:.1}× 提速，{} 个可行村庄）",
        d1,
        d2,
        d1.as_secs_f64() / d2.as_secs_f64(),
        par.len()
    );
}
