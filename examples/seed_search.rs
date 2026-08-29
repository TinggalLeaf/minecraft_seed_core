//! 种子搜索：find_biomes / find_structures / find_biomes_with_structure。
//!
//! 运行：cargo run --release --example seed_search

use minecraft_seed_core::search::{find_biomes, find_biomes_with_structure, find_structures};
use minecraft_seed_core::{Dimension, McVersion, StructureType};

fn main() {
    let mc = McVersion::V1_20;

    // 1. 找「(0,0) 是平原」的第一个种子
    let s = find_biomes(mc, Dimension::Overworld, &[1], 0, 0, 1, 1, 320, 0);
    println!("第一个 (0,0) 为平原的种子: {s}");

    // 2. 找「16x16 区域同时含丛林和恶地」的种子
    let s = find_biomes(mc, Dimension::Overworld, &[21, 37], -8, -8, 16, 16, 320, 0);
    println!("第一个 16x16 内含丛林+恶地的种子: {s}");

    // 3. 找「region(0,0) 的村庄候选在原点 ±16 方块内且群系可行」的种子
    //    （返回值 = (高16位 << 48) | 位置基值，与 mcseedmap.com 一致）
    let full = find_structures(mc, Dimension::Overworld, StructureType::Village, 0, 0, 16, 0);
    println!("村庄贴出生点的打包种子: {full}（低48位基值 {}）", full & 0xFFFFFFFFFFFF);

    // 4. 村庄 + 区域内必须是平原
    let full = find_biomes_with_structure(
        mc,
        Dimension::Overworld,
        StructureType::Village,
        &[1],
        0,
        0,
        16,
        320,
        0,
    );
    println!("村庄+平原的打包种子: {full}");
}
