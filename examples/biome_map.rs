//! 群系 ASCII 图示例：生成一块 1:4 比例的主世界群系区域并打印字符画。
//!
//! 运行：`cargo run --example biome_map`
use minecraft_seed_core::generator::{Generator, Range};
use minecraft_seed_core::{BiomeId, Dimension, McVersion};

/// 常见群系的字符映射（未列出的用 '?'）。
fn glyph(b: BiomeId) -> char {
    use BiomeId::*;
    match b {
        Ocean | FrozenOcean | WarmOcean | LukewarmOcean | ColdOcean => '~',
        DeepOcean | DeepWarmOcean | DeepLukewarmOcean | DeepColdOcean | DeepFrozenOcean => 'O',
        River | FrozenRiver => '=',
        Plains | SunflowerPlains | Meadow => '.',
        Forest | FlowerForest | BirchForest | TallBirchForest | DarkForest | CherryGrove
        | PaleGarden => 'T',
        Taiga | SnowyTaiga | GiantTreeTaiga | GiantSpruceTaiga => 't',
        Jungle | BambooJungle => 'J',
        Swamp | MangroveSwamp => 's',
        Desert => 'd',
        Savanna | SavannaPlateau => 'v',
        Badlands | ErodedBadlands | WoodedBadlandsPlateau => 'b',
        Mountains | WoodedMountains | GravellyMountains | StonyPeaks => '^',
        JaggedPeaks | FrozenPeaks => 'A',
        SnowyTundra | SnowyMountains | IceSpikes | SnowySlopes | Grove | SnowyBeach => '*',
        MushroomFields | MushroomFieldShore => 'M',
        Beach | StoneShore => ',',
        _ => '?',
    }
}

fn main() {
    let seed: u64 = 12345;
    let mc = McVersion::V1_20;
    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);

    // 96x48 个 1:4 单元 = 384x192 方块，中心在世界原点。
    // 1.18+ 群系随 y 变化：y 取 319>>2（对应方块 y≈319 的地表高度采样，
    // 与 isViableStructurePos 对地表结构的采样高度一致）。
    let r = Range::new(4, -48, -24, 96, 48).with_y(319 >> 2, 1);
    let cells = g.gen_biomes(r);

    println!("seed={seed} mc={} {}x{} cells @ scale 1:4 (y=79)", mc.name(), r.sx, r.sz);
    println!("legend: ~=ocean O=deep_ocean ==river .=plains T=forest t=taiga J=jungle s=swamp");
    println!("        d=desert v=savanna b=badlands ^=mountain A=peaks *=snowy M=mushroom ,=shore");
    for z in 0..r.sz {
        // 输出索引为 out[i_y*sx*sz + i_z*sx + i_x]
        let line: String = (0..r.sx).map(|x| glyph(cells[(z * r.sx + x) as usize])).collect();
        println!("{line}");
    }
}
