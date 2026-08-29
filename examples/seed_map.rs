//! chunkbase seed-map 综合示例：复刻网站主界面的全部数据图层。
//!
//! 对应 https://www.chunkbase.com/apps/seed-map 的功能：
//! - 出生点（spawn）
//! - 要塞（strongholds）
//! - 史莱姆区块（slime chunks）
//! - 群系地图（biomes，含下界/末地）
//! - 结构图层（村庄/神殿/堡垒/堡垒遗迹/末地城/化石/废弃营地……）
//!
//! 运行：`cargo run --example seed_map`
use minecraft_seed_core::generator::Generator;
use minecraft_seed_core::structure::{
    self, get_config, get_structure_pos, is_viable_structure_pos, scan_fossils, StrongholdIter,
};
use minecraft_seed_core::{BiomeId, Dimension, McVersion, Range, StructureType};

const SEED: u64 = 1085393142614036966;
const MC: McVersion = McVersion::V1_21; // chunkbase java_26.3 与 1.21.4 算法一致

fn main() {
    println!("=== chunkbase seed-map 功能总览 ===");
    println!("seed={SEED}  version={} ({MC:?})\n", MC.name());

    // ---- 出生点 ----
    let g = Generator::new(MC).with_seed(Dimension::Overworld, SEED);
    let est = structure::estimate_spawn(&g);
    let exact = structure::get_spawn(&g);
    println!("出生点：估计 ({}, {})，精确 ({}, {})", est.x, est.z, exact.x, exact.z);

    // ---- 要塞（前 3 座，1.9+ 共 128 座） ----
    let mut sh = StrongholdIter::new(MC, SEED);
    print!("要塞：");
    for _ in 0..3 {
        sh.next(Some(&g));
        print!("({}, {})  ", sh.pos.x, sh.pos.z);
    }
    println!();

    // ---- 史莱姆区块（出生点附近 3×3） ----
    let slime: Vec<(i32, i32)> = (exact.x.div_euclid(16) - 1..=exact.x.div_euclid(16) + 1)
        .flat_map(|cx| {
            (exact.z.div_euclid(16) - 1..=exact.z.div_euclid(16) + 1)
                .filter(move |&cz| structure::is_slime_chunk(SEED, cx, cz))
                .map(move |cz| (cx, cz))
        })
        .collect();
    println!("出生点附近史莱姆区块：{slime:?}");

    // ---- 群系采样（三维度） ----
    let biomes = g.gen_biomes(Range::new(4, 0, 0, 1, 1).with_y(320 / 4, 1));
    let b0 = BiomeId::from_i32(biomes[0] as i32);
    println!("主世界原点群系：{b0:?}");
    let gn = Generator::new(MC).with_seed(Dimension::Nether, SEED);
    let bn = gn.gen_biomes(Range::new(4, -477 / 4, -525 / 4, 1, 1).with_y(64 / 4, 1));
    println!("下界 (-477,-525) 群系：{:?}", BiomeId::from_i32(bn[0] as i32));

    // ---- 结构图层：以 (-477, -525) 为中心扫描各类结构 ----
    println!("\n下界 (-477,-525) 周边结构：");
    for stype in [StructureType::Bastion, StructureType::Fortress, StructureType::RuinedPortalN] {
        let Some(conf) = get_config(stype, MC) else { continue };
        let mut found = None;
        let (rcx, rcz) = (-477i32.div_euclid(conf.region_size * 16),
                          -525i32.div_euclid(conf.region_size * 16));
        'outer: for rz in rcz - 2..=rcz + 2 {
            for rx in rcx - 2..=rcx + 2 {
                if let Some(pos) = get_structure_pos(stype, MC, SEED, rx, rz) {
                    if is_viable_structure_pos(stype, &gn, pos.x, pos.z, 0) != 0 {
                        found = Some(pos);
                        break 'outer;
                    }
                }
            }
        }
        match found {
            Some(p) => println!("  {stype:?}: ({}, {})", p.x, p.z),
            None => println!("  {stype:?}: 附近无"),
        }
    }

    // ---- 化石（1.20+，逐区块散布，vanilla 限沙漠/沼泽群系） ----
    let fossils = scan_fossils(MC, SEED, -8, -8, 7, 7);
    println!("\n出生点 ±128 区块内化石候选：{} 处（前 5）", fossils.len());
    for p in fossils.iter().take(5) {
        println!("  fossil @ ({}, {})", p.x, p.z);
    }

    // ---- 废弃营地（26.3-s1+，网站 UI 门控；算法层 1.21.4+） ----
    let camp = get_config(StructureType::AbandonedCamp, MC).unwrap();
    let mut camps = Vec::new();
    for rz in -1..=1 {
        for rx in -1..=1 {
            if let Some(p) = get_structure_pos(StructureType::AbandonedCamp, MC, SEED, rx, rz) {
                camps.push(p);
            }
        }
    }
    println!("\n废弃营地（region={}, salt={}）原点附近：", camp.region_size, camp.salt);
    for p in &camps {
        println!("  camp @ ({}, {})", p.x, p.z);
    }
}
