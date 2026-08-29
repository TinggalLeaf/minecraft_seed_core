//! 结构部件生成：末地城部件树、下界堡垒部件、旧版村庄房屋列表、结构变体。
//!
//! 运行：cargo run --example structure_pieces

use minecraft_seed_core::structure::{
    get_end_city_pieces, get_fortress_pieces, get_house_list, get_structure_pos, get_variant,
    is_viable_end_city_terrain, is_viable_structure_pos, StructureType, end_city,
};
use minecraft_seed_core::{Dimension, Generator, McVersion};

#[allow(clippy::collapsible_if)]
fn main() {
    let seed = 12345u64;

    // 找一个 1.20 真实可生成的末地城
    let mc = McVersion::V1_20;
    let g = Generator::new(mc).with_seed(Dimension::End, seed);
    let mut found = None;
    'outer: for rx in 10..30 {
        for rz in 10..30 {
            if let Some(pos) = get_structure_pos(StructureType::EndCity, mc, seed, rx, rz) {
                if is_viable_structure_pos(StructureType::EndCity, &g, pos.x, pos.z, 0) != 0
                    && is_viable_end_city_terrain(&g, g.surface_noise(), pos.x, pos.z) != 0
                {
                    found = Some(pos);
                    break 'outer;
                }
            }
        }
    }
    let pos = found.expect("未找到末地城");
    println!("末地城 @ ({}, {})", pos.x, pos.z);
    let pieces = get_end_city_pieces(seed, pos.x >> 4, pos.z >> 4);
    println!("部件数: {}（含 {} 种类型）", pieces.len(), {
        let mut t: Vec<i32> = pieces.iter().map(|p| p.piece_type).collect();
        t.sort();
        t.dedup();
        t.len()
    });
    if let Some(ship) = pieces.iter().find(|p| p.piece_type == end_city::END_SHIP) {
        println!("末地船 @ ({}, {}, {})", ship.pos.x, ship.pos.y, ship.pos.z);
    }

    // 下界堡垒
    let mc = McVersion::V1_16;
    let g = Generator::new(mc).with_seed(Dimension::Nether, seed);
    let mut fpos = None;
    'outer2: for rx in 0..20 {
        for rz in 0..20 {
            if let Some(pos) = get_structure_pos(StructureType::Fortress, mc, seed, rx, rz) {
                if is_viable_structure_pos(StructureType::Fortress, &g, pos.x, pos.z, 0) != 0 {
                    fpos = Some(pos);
                    break 'outer2;
                }
            }
        }
    }
    if let Some(pos) = fpos {
        let pieces = get_fortress_pieces(mc, seed, pos.x >> 4, pos.z >> 4);
        println!("\n下界堡垒 @ ({}, {}): {} 个部件", pos.x, pos.z, pieces.len());
    }

    // 旧版村庄（1.12）房屋列表
    let mc = McVersion::V1_12;
    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);
    let mut vpos = None;
    'outer3: for rx in -10..10 {
        for rz in -10..10 {
            if let Some(pos) = get_structure_pos(StructureType::Village, mc, seed, rx, rz) {
                if is_viable_structure_pos(StructureType::Village, &g, pos.x, pos.z, 0) != 0 {
                    vpos = Some(pos);
                    break 'outer3;
                }
            }
        }
    }
    if let Some(pos) = vpos {
        let hl = get_house_list(seed, pos.x >> 4, pos.z >> 4);
        println!("\n1.12 村庄 @ ({}, {}) 房屋数: {:?}", pos.x, pos.z, hl.houses);
        // 变体（朝向/群系变体）
        if let Some(v) = get_variant(StructureType::Village, mc, seed, pos.x, pos.z, 1) {
            println!("村庄变体: rotation={} biome={}", v.rotation, v.biome);
        }
    }
}
