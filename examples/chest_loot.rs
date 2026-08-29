//! 宝箱内容预测示例：给定世界种子与坐标，预测结构宝箱的战利品。
//!
//! 场景取自 chunkbase seed-map 链接：
//!   seed=1085393142614036966, platform=java_26.3, dimension=nether,
//!   坐标 (-477, -525) 附近。
//!
//! 流程：
//! 1. 用结构查找定位 (-477,-525) 附近的堡垒遗迹（Bastion）与下界要塞
//!    （Fortress）；
//! 2. 用 `loot` 模块按 1.20.1 战利品表预测宝箱内容。
//!
//! 运行：`cargo run --example chest_loot`
//!
//! 注意：战利品表数据当前为 1.20.1 快照（loot 表在 1.21 中未改箱子
//! 内容）；预测的箱子坐标取结构起始点，实际箱子在结构内部由部件
//! 生成决定，坐标不同则内容不同——本示例演示的是完整的
//! 「种子 + 坐标 → 宝箱内容」链路用法。
use minecraft_seed_core::generator::Generator;
use minecraft_seed_core::loot::{self, LootVersion};
use minecraft_seed_core::structure::{
    get_config, get_structure_pos, is_viable_structure_pos,
};
use minecraft_seed_core::{Dimension, McVersion, StructureType};

const SEED: u64 = 1085393142614036966;

/// 在 (x, z) 附近的 region 范围内找指定结构的可行位置。
fn find_nearby(
    stype: StructureType,
    mc: McVersion,
    dim: Dimension,
    x: i32,
    z: i32,
) -> Option<(i32, i32)> {
    let conf = get_config(stype, mc)?;
    let g = Generator::new(mc).with_seed(dim, SEED);
    let (rcx, rcz) = (x.div_euclid(conf.region_size * 16), z.div_euclid(conf.region_size * 16));
    for rz in rcz - 1..=rcz + 1 {
        for rx in rcx - 1..=rcx + 1 {
            let Some(pos) = get_structure_pos(stype, mc, SEED, rx, rz) else {
                continue;
            };
            if is_viable_structure_pos(stype, &g, pos.x, pos.z, 0) != 0 {
                return Some((pos.x, pos.z));
            }
        }
    }
    None
}

fn show_chest(chest: &str, x: i32, y: i32, z: i32) {
    let v = LootVersion::V1_20_1;
    match loot::predict_chest(v, chest, SEED as i64, x, y, z) {
        Ok(items) if items.is_empty() => println!("    (空箱子)"),
        Ok(items) => {
            for it in items {
                let ench = if it.enchanted { "（附魔）" } else { "" };
                println!("    {:>2} × {}{}", it.count, it.item, ench);
            }
        }
        Err(e) => println!("    错误：{e}"),
    }
}

fn main() {
    let mc = McVersion::V1_21; // chunkbase 的 java_26.3 与 1.21.4 算法一致
    println!("seed={SEED}  mc={} 维度=下界 目标=(-477, -525)\n", mc.name());

    // 1) 找最近的堡垒遗迹，预测四种堡垒宝箱
    if let Some((x, z)) = find_nearby(StructureType::Bastion, mc, Dimension::Nether, -477, -525) {
        println!("堡垒遗迹 @ ({x}, {z})");
        for chest in [
            "bastion_treasure",
            "bastion_bridge",
            "bastion_hoglin_stable",
            "bastion_other",
        ] {
            println!("  [{chest}]");
            show_chest(chest, x, 64, z);
        }
    } else {
        println!("附近没有可行堡垒遗迹");
    }

    // 2) 找最近的下界要塞，预测 nether_bridge 宝箱
    if let Some((x, z)) = find_nearby(StructureType::Fortress, mc, Dimension::Nether, -477, -525) {
        println!("\n下界要塞 @ ({x}, {z})");
        println!("  [nether_bridge]");
        show_chest("nether_bridge", x, 64, z);
    } else {
        println!("\n附近没有可行下界要塞");
    }

    // 3) 主世界对照：废弃传送门宝箱（主世界侧坐标约为下界 ×8）
    let g = Generator::new(mc).with_seed(Dimension::Overworld, SEED);
    let conf = get_config(StructureType::RuinedPortal, mc).unwrap();
    let (rx, rz) = ((-477i32 * 8).div_euclid(conf.region_size * 16),
                    (-525i32 * 8).div_euclid(conf.region_size * 16));
    if let Some(pos) = get_structure_pos(StructureType::RuinedPortal, mc, SEED, rx, rz) {
        if is_viable_structure_pos(StructureType::RuinedPortal, &g, pos.x, pos.z, 0) != 0 {
            println!("\n废弃传送门（主世界）@ ({}, {})", pos.x, pos.z);
            println!("  [ruined_portal]");
            show_chest("ruined_portal", pos.x, 64, pos.z);
        }
    }
}
