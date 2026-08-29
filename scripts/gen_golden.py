# -*- coding: utf-8 -*-
"""生成 Rust golden data 代码块（用于 `tests/loot_golden_data.rs`）。

每个用例包含：loot_table_id、world_seed、(x,y,z)、items（item/count/enchanted）。
"""
import io
import os
import sys

SOURCE_PROJECT = r"E:\Projects\Minecraft\宝箱内容生成"
os.chdir(SOURCE_PROJECT)
sys.path.insert(0, SOURCE_PROJECT)

from src.loot_predictor import predict, ALL_TABLES  # noqa
from src.structure_seeds import chest_seed_for
from src.loot_table import load_loot_table  # noqa: re-exported for our direct path

CASES = [
    ("minecraft:chests/ruined_portal",            12345, 100, 64, 200),
    ("minecraft:chests/village/village_weaponsmith", 12345, 100, 64, 200),
    ("minecraft:chests/buried_treasure",          12345, 100, 64, 200),
    ("minecraft:chests/nether_bridge",            987654321, 1234, 70, -5678),
    ("minecraft:chests/desert_pyramid",           12345, 100, 64, 200),
    ("minecraft:chests/stronghold_library",       42, -200, 30, 1500),
    ("minecraft:chests/end_city_treasure",        12345, 100, 64, 200),
    ("minecraft:chests/bastion_bridge",           12345, 100, 64, 200),
    ("minecraft:chests/ancient_city",             12345, 100, 64, 200),
    ("minecraft:chests/shipwreck_treasure",       12345, 100, 64, 200),
    ("minecraft:chests/igloo_chest",              12345, 100, 64, 200),
    ("minecraft:chests/pillager_outpost",         12345, 100, 64, 200),
    ("minecraft:chests/ruined_portal",            -1, -100, 64, -200),
    ("minecraft:chests/simple_dungeon",           12345, 100, 64, 200),
    ("minecraft:chests/jungle_temple",            12345, 100, 64, 200),
    ("minecraft:chests/bastion_treasure",         12345, 100, 64, 200),
    ("minecraft:chests/abandoned_mineshaft",      12345, 100, 64, 200),
    ("minecraft:chests/village/village_toolsmith", 12345, 100, 64, 200),
    ("minecraft:chests/stronghold_crossing",      42, -200, 30, 1500),
    ("minecraft:chests/stronghold_corridor",      42, -200, 30, 1500),
    ("minecraft:empty",                            0, 0, 0, 0),
    ("minecraft:gameplay/fishing",                 12345, 0, 0, 0),
    # blocks（覆盖 set_count=0、apply_bonus、alternatives 等新路径）
    ("minecraft:blocks/diamond_ore",               12345, 100, 64, 200),
    ("minecraft:blocks/oak_leaves",                12345, 100, 64, 200),
    ("minecraft:blocks/grass_block",               42, -200, 30, 1500),
    ("minecraft:blocks/ancient_debris",            987654321, 1234, 70, -5678),
    # entities
    ("minecraft:entities/zombie",                  12345, 100, 64, 200),
    ("minecraft:entities/sheep/blue",              12345, 100, 64, 200),
    ("minecraft:entities/cow",                     42, -200, 30, 1500),
    ("minecraft:entities/wither_skeleton",         987654321, 1234, 70, -5678),
    # gameplay / archaeology
    ("minecraft:gameplay/piglin_bartering",        12345, 100, 64, 200),
    ("minecraft:archaeology/desert_pyramid",       12345, 100, 64, 200),
]

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

cases_rust = []
for (table_id, ws, x, y, z) in CASES:
    buf = io.StringIO()
    old = sys.stdout
    sys.stdout = buf
    try:
        if table_id == "minecraft:empty" or not table_id.startswith("minecraft:chests/"):
            from src.rng import XoroshiroRandomSource
            sys.stdout = old
            # 与 chest 路径同：先调 chest_seed_for 拿到同样的 seed，再走
            # `rng = Xoroshiro(seed); rng.next_long()` 的流程。
            chest_seed = chest_seed_for(table_id, ws, x, y, z)
            rng = XoroshiroRandomSource(chest_seed)
            rng.next_long()
            table = load_loot_table("data/" + ALL_TABLES[table_id])
            stacks = table.generate(rng, 0.0)
            items = [(s.item_id, s.count, "__enchanted__" in (s.nbt or {})) for s in stacks]
        else:
            predict(ws, x, y, z, table_id, samples=1)
            sys.stdout = old
            output = buf.getvalue()
            chest_seed = chest_seed_for(table_id, ws, x, y, z)
            # 直接调 generate()，不要只解析打印输出（打印里丢了 enchanted 标记）。
            from src.rng import XoroshiroRandomSource
            rng = XoroshiroRandomSource(chest_seed)
            rng.next_long()
            table = load_loot_table("data/" + ALL_TABLES[table_id])
            stacks = table.generate(rng, 0.0)
            items = [(s.item_id, s.count, "__enchanted__" in (s.nbt or {})) for s in stacks]
    finally:
        sys.stdout = old
    cases_rust.append({
        "id": table_id,
        "ws": ws,
        "x": x,
        "y": y,
        "z": z,
        "seed": chest_seed,
        "items": items,
    })

print("// 由 `scripts/gen_golden.py` 自动生成，请勿手改。")
print(f"// {len(cases_rust)} 组用例覆盖 5 种 seed 推导模式 + enchant_randomly / empty /")
print("// 非 chest 表（gameplay）+ blocks / entities / archaeology。")
print()
print("/// 单组用例：(loot_table_id, world_seed, x, y, z, seed, items)。")
print("pub(super) struct GoldenCase {")
print("    pub id: &'static str,")
print("    pub world_seed: i64,")
print("    pub x: i32,")
print("    pub y: i32,")
print("    pub z: i32,")
print("    pub expected_seed: u64,")
print("    pub expected_items: &'static [(&'static str, i32, bool)],")
print("}")
print()
print("pub(super) static GOLDEN_CASES: &[GoldenCase] = &[")
for c in cases_rust:
    seed = c["seed"]
    if seed < 0:
        seed_lit = f"0x{seed & 0xFFFFFFFFFFFFFFFF:016X}u64"
    else:
        seed_lit = f"{seed}u64"
    print(f"    GoldenCase {{")
    print(f"        id: \"{c['id']}\",")
    print(f"        world_seed: {c['ws']}i64,")
    print(f"        x: {c['x']},")
    print(f"        y: {c['y']},")
    print(f"        z: {c['z']},")
    print(f"        expected_seed: {seed_lit},")
    print(f"        expected_items: &[")
    for it, cnt, ench in c["items"]:
        print(f"            (\"{it}\", {cnt}, {str(ench).lower()}),")
    print(f"        ],")
    print(f"    }},")
print("];")
