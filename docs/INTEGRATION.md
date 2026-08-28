# minecraft_seed_core 对接文档

本文档面向要把本库接入自己程序（种子查找器、地图渲染器、Discord bot 等）的开发者。所有 API 名称与 `src/` 源码一致；示例代码均已验证可编译（见 `examples/`）。

## 模块地图

| 模块 | 内容 | 对应 cubiomes |
| --- | --- | --- |
| `version` | `McVersion`、`Dimension` | `MCVersion` / `Dimension` |
| `biome` | `BiomeId` 枚举与分类/海洋/突变等谓词 | `biomes.h` / `biomes.c` |
| `rng` | `JavaRandom`（48 位 LCG）、`Xoroshiro`、种子流水线（`layer_salt`/`start_seed`/`chunk_seed`） | `java.h` / `xoro.h` / `layers.c` 的种子助手 |
| `noise` | `PerlinNoise`、`OctaveNoise`、`DoublePerlinNoise`、`BiomeNoise`、`SurfaceNoise`、`beta`（`BiomeNoiseBeta`/`SurfaceNoiseBeta`/`get_old_beta_biome`） | `noise.c` / `biomenoise.c` |
| `generator` | `Generator`、`Range`，按版本分派（beta 气候噪声 / `layers` / `v1_18` / `nether` / `end` / `voronoi`），`map_approx_height` 地表高度近似（`ApproxHeight`） | `generator.c` / `layers.c` |
| `structure` | `StructureType`、`get_config`、`get_structure_pos`、`is_viable_structure_pos`、`get_variant`、`StrongholdIter`、`estimate_spawn`、`get_spawn`、`is_viable_structure_terrain`、`is_viable_end_city_terrain`、`is_end_chunk_empty`、`get_linked_gateway_chunk`、`get_linked_gateway_pos`、`is_slime_chunk` 等 | `finders.c` / `finders.h` |

crate 根部 re-export：`BiomeId`、`Generator`、`Range`、`StructureType`、`Dimension`、`McVersion`。

## 版本与维度

```rust
use minecraft_seed_core::{McVersion, Dimension};

let mc = McVersion::V1_20;
assert_eq!(mc.name(), "1.20.6");           // 人类可读字符串
assert!(mc >= McVersion::V1_18);           // 枚举按发布时间排序，可直接比较
assert!(mc.has_multi_noise_biomes());      // 1.18+ 为 true
for v in McVersion::ALL { /* 28 个版本，升序 */ }
```

- `V1_X` 表示该大版本的**最新补丁**（与 cubiomes 一致），如 `V1_16` = 1.16.5；规则有差异的小版本单列：`V1_16_1`、`V1_19_2`、`V1_21_1`、`V1_21_3`；Beta 版本为 `B1_7`（"b1.7.3"）、`B1_8`（"b1.8.1"）。
- `McVersion::NEWEST` / `McVersion::OLDEST` 给出支持边界（当前为 1.21.4 / b1.7.3）。
- `Dimension`：`Nether = -1`、`Overworld = 0`、`End = 1`（判别值与 cubiomes 相同）。

## BiomeId 与 i32 互转

```rust
use minecraft_seed_core::BiomeId;

let b = BiomeId::Plains;
let id: i32 = b as i32;                    // 1（判别值与 cubiomes 一致，含 128+ 突变变体）
let b2 = BiomeId::from_i32(id);            // Some(Plains)；未知值（54、176、999…）返回 None
assert!(BiomeId::CherryGrove.exists_in(McVersion::V1_20));
assert!(!BiomeId::DesertHills.exists_in(McVersion::V1_18)); // 1.18 起移除的变体
```

注意 `BiomeId::None = -1` 是合法枚举值（cubiomes 的 `none`），`from_i32(-1)` 返回 `Some(BiomeId::None)` 而非 `Option::None`。

辅助谓词（作用于原始 `i32`）：`is_oceanic` / `is_deep_ocean` / `is_shallow_ocean` / `is_mesa` / `is_snowy` / `get_category` / `get_mutated` / `are_similar`。结构侧还有 `structure::biome_exists` / `structure::is_overworld`。

## Generator 完整用法

### 生命周期

```rust
use minecraft_seed_core::{Dimension, Generator, McVersion};

let g = Generator::new(McVersion::V1_20)                 // setupGenerator（不含种子）
    .with_large_biomes(true)                             // 可选；必须在 with_seed 之前
    .with_seed(Dimension::Overworld, 12345u64);          // applySeed，可重复调用换种子/维度
```

- `Generator::new(mc)`：1.18+ 主世界在此构建群系噪声 spline 表；B1.7- 构建 beta 气候噪声占位；B1.8–1.17 的分层层栈推迟到 `with_seed`（需要 large 标志）。
- `with_large_biomes(large)`：只影响主世界（对应 C `setupGenerator` 的 `LARGE_BIOMES` flag；B1.7- 无此世界类型，设置无效）。
- `with_seed(dim, seed)`：可重复调用更换维度/种子，复用已构建的噪声表。
- 只读访问器：`dim()` / `seed()` / `version()` / `biome_noise()`（1.18+ 主世界的噪声，调试用）。
- `Generator: Clone`（分层路径内部含 `Cell`，因此不是 `Sync`——可以移动进别的线程，但不能跨线程共享引用；多线程找种请每个线程各建一份）。

### 坐标语义（重点）

- `get_biome(x, y, z)`：**三个坐标都是 1:4 群系比例**，即方块坐标除以 4（`x_block >> 2`）。
- `gen_biomes(Range)`：`Range { scale, x, z, sx, sz, y, sy }`
  - `scale`：水平比例因子，支持 **1、4、16、64、256**（B1.7- 主世界例外：支持任意 2 的幂，含 1 和 2）。scale 4 即默认群系比例；scale 1 是方块级（B1.8–1.17 走 voronoi 扰动，B1.7- 由 beta 噪声直接生成）。
  - `x, z`：区域西北角，**按 scale 比例**计。即 scale=4 时 `x=0` 对应方块 0，scale=1 时 `x` 就是方块坐标。
  - `y, sy`：垂直位置与尺寸。**scale != 1 时垂直比例恒为 1:4**（与水平 scale 无关），即 `y = y_block >> 2`；`sy <= 0` 视为 1。`Range::new(scale, x, z, sx, sz)` 是 2D 便捷构造（`y=0, sy=1`），`.with_y(y, sy)` 设垂直范围。
  - 输出长度 `sx*sy*sz`，索引 `out[i_y*sx*sz + i_z*sx + i_x]`。
- **y 的作用**：仅 1.18+ 主世界的群系随 y 变化（洞穴群系等）。1.17 及更早的主世界完全忽略 y（生成 2D 平面后沿 y 复制）；下界群系实际上也不随 y 变化。查地表群系的惯例采样高度是 `y = 319 >> 2`（与 `is_viable_structure_pos` 对地表结构的采样一致）；查海底神殿等用海床附近（`y = 36 >> 2`）。

```rust
use minecraft_seed_core::generator::Range;

// 单点：方块 (512, 64, -128) 处的群系
let b = g.get_biome(512 >> 2, 64 >> 2, -128 >> 2);

// 区域：方块 [-256, 256)² 的 1:4 群系图
let area = g.gen_biomes(Range::new(4, -64, -64, 128, 128));

// 体积：1.18+ 的 3D 群系（如找深暗之域）
let vol = g.gen_biomes(Range::new(4, -64, -64, 128, 128).with_y(-64 >> 2, 96 >> 2));
```

### 维度行为

- `Dimension::Overworld`：全版本支持（Beta 1.7–1.21）。B1.7- 走气候噪声路径（`noise::beta`），scale 支持任意 2 的幂；B1.8–1.17 走分层层栈，scale 支持 1/4/16/64/256。
- `Dimension::Nether`：1.16.1+ 真实多噪声；**1.15 及更早**不报错，整个区域填充 `BiomeId::NetherWastes`（与 cubiomes 行为一致）。scale 支持 1/4/16/64/256（`scale <= 0` 视为 4）。
- `Dimension::End`：1.9+ simplex 高地噪声；**1.8 及更早**填充 `BiomeId::TheEnd`。scale 支持 4/16/64/256 及更大；**scale 1 未移植，调用 panic**。

### Panics（调用方需要避免的输入）

- 未调用 `with_seed` 就 `gen_biomes` / `get_biome`。
- B1.8–1.17 主世界 `scale` 不是 1/4/16/64/256（B1.7- 接受任意 2 的幂，其他值 panic）。
- 末地 `scale == 1`。
- `StrongholdIter::next` 在 B1.8–1.19.2 传 `None`（需要主世界生成器做群系检查）。
- `is_viable_structure_pos` 用于 B1.7- 主世界（C 对 beta 只做了半成品支持，这里同样不可用）。

## 结构查找完整流程

### 标准三步

```rust
use minecraft_seed_core::structure::{
    get_config, get_structure_pos, is_viable_structure_pos, get_variant,
};
use minecraft_seed_core::{Dimension, Generator, McVersion, StructureType};

let mc = McVersion::V1_20;
let seed = 12345u64;
let stype = StructureType::DesertPyramid;

// 1) 配置：该版本不支持此结构时返回 None（如 1.13+ 的 Feature、1.9 前的 Igloo）
let conf = get_config(stype, mc).expect("unsupported");
// conf: salt / region_size(region 边长,区块) / chunk_range(region 内偏移范围,区块)
//       struct_type / dim / rarity

// 2) 候选位置：按 region 网格扫描。region 坐标与方块坐标的换算：
//    reg = floor_div(block, region_size * 16)
let (reg_x, reg_z) = (0, 0);
if let Some(pos) = get_structure_pos(stype, mc, seed, reg_x, reg_z) {
    // pos 是方块坐标。候选只依赖【种子低 48 位】与 region 坐标；
    // 有些 region 无论群系如何都不生成（稀有度判定），此时返回 None。

    // 3) 群系可行性：需要按【结构的维度】初始化的 Generator
    let g = Generator::new(mc).with_seed(Dimension::Overworld, seed);
    let v = is_viable_structure_pos(stype, &g, pos.x, pos.z, 0);
    if v != 0 {
        // v 通常为 1；村庄等类型返回可行的群系 ID（与 C 一致），
        // 可直接作为 get_variant 的 biome_id 参数
        let sv = get_variant(stype, mc, seed, pos.x, pos.z, v);
    }
}
```

要点：

- **顺序不可换**：`get_config` 决定 region 网格参数 → `get_structure_pos` 算候选 → `is_viable_structure_pos` 验群系。`get_structure_pos` 内部自己会调 `get_config`，不要求你先调，但扫描范围时需要配置里的 `region_size`。
- `is_viable_structure_pos(stype, g, x, z, flags)`：`x, z` 是 `get_structure_pos` 输出的**方块坐标**；`flags` 为结构特定过滤（村庄的群系变体 ID，0 = 不限）。返回 `0` = 不可行，非 0 = 可行（部分类型为群系 ID）。
- Generator 的维度必须与结构维度一致：`StructureConfig.dim`（-1 下界 / 0 主世界 / 1 末地）。注意 1.16.1–1.17 的下界废弃传送门用 `RuinedPortalN`，1.18+ 下界 portal 的配置记为 `RuinedPortal` 但 `dim = -1`（这是 cubiomes 的原样行为）。
- **要塞（Fortress）1.18+ 的可行性与群系联动**（生成在堡垒遗迹不生成处），`is_viable_structure_pos` 已内部处理，直接调用即可。
- B1.8–1.17 主世界的 viability 带**粗层剪枝**模拟（C 的 `mapViableBiome`/`mapViableShore`），结果与 C 逐位一致，包括那些"细层有目标群系但粗层没有而判不可行"的边角情形。B1.7- 主世界不可用（C 对 beta 只做了半成品支持），调用会 panic。

### 变体判定 `get_variant`

```rust
let sv = get_variant(StructureType::Village, mc, seed, pos.x, pos.z, biome_id);
```

- `x, z`：结构候选的方块坐标（`get_structure_pos` 的输出）。
- `biome_id`：群系变体提示（村庄必填，通常用 `is_viable_structure_pos` 的返回值）；其他类型传 `-1`。
- 返回 `Option<StructureVariant>`：不支持该类型 / 不可生成（如紫晶洞稀有度未过）时返回 `None`。
- 支持的类型：`Village`（含僵尸村庄 `abandoned`、朝向 `rotation`、包围盒）、`Bastion`（4 种 `start`）、`AncientCity`、`RuinedPortal`/`RuinedPortalN`（`giant`/`underground`/`airpocket`）、`DesertPyramid`/`JungleTemple`/`SwampHut`（1.20+ 含朝向）、`Igloo`（`basement`/`size`）、`Monument`（固定包围盒）、`Geode`（`y`/`size`/`cracked`）、`TrialChambers`。
- `rotation`：`0=0°, 1=cw90, 2=cw180, 3=cw270`；`x/z/sx/sy/sz` 为相对包围盒。

### 结构部件生成

对应 cubiomes `finders.c` 的部件级 API：给定**结构所在区块**生成该结构的
部件布局。返回 `Vec<Piece>`（`name` / `pos` / `bb0` / `bb1` / `rot` /
`depth` / `piece_type`，与 C 的 `Piece` 逐字段一致，去掉 `next` 指针）。

```rust
use minecraft_seed_core::structure::{
    get_end_city_pieces, get_fortress_pieces, get_house_list,
    end_city, fortress, house,
};

// 末地城部件树（1.9+；与版本无关，输入为真实末地城区块）
let pieces = get_end_city_pieces(seed, chunk_x, chunk_z);
for p in &pieces {
    if p.piece_type == end_city::END_SHIP {
        println!("末影船 @ {:?}..{:?}", p.bb0, p.bb1);
    }
}

// 下界堡垒布局（自动按 mc 选择 ≤1.15 / 1.16+ 随机源路径）
let pieces = get_fortress_pieces(McVersion::V1_20, seed, chunk_x, chunk_z);
let spawners = pieces.iter()
    .filter(|p| p.piece_type == fortress::BRIDGE_SPAWNER)
    .count();

// 1.13 及更早村庄的房屋列表
let hl = get_house_list(seed, chunk_x, chunk_z);
println!("铁匠铺 {} 座", hl.houses[house::BLACKSMITH]);
// hl.rng_state 为 C 原样返回的 48 位 LCG 状态
```

要点：

- **输入必须是真实可生成的结构区块**：先按「标准三步」用
  `get_structure_pos` + `is_viable_structure_pos` 找到结构（末地城再加
  `is_viable_end_city_terrain` 判地形），然后取 `pos.x >> 4, pos.z >> 4`
  作为 `chunk_x, chunk_z`。对不存在结构的坐标调用不会报错，但输出无意义。
- 部件类型常量分三个模块：`end_city::*`（20 种，含 `END_SHIP`）、
  `fortress::*`（15 种，含 `FORTRESS_START`/`FORTRESS_END`）、
  `house::*`（9 种房屋下标）。
- `rot` 朝向：`0=北 1=东 2=南 3=西`；末地城的 `depth` 是递归批次的
  生代编号（含 C 的 `int8_t` 截断语义，可能为负），堡垒的 `depth` 是
  距起始部件的延伸深度（≤30）。
- 与 C 的差异仅内存管理：C 要求调用方提供定长缓冲（末地城
  `END_CITY_PIECES_MAX = 421`，对应常量 `end_city::PIECES_MAX`），
  这里返回 `Vec`；堡垒的待处理队列用保序 `Vec<usize>` 复刻 C 链表的
  随机抽取/尾部追加语义，随机数消费顺序与 C 完全一致。
- golden 验证见 `tests/bundle_c_golden.rs`（真实结构区块 × 多版本 ×
  多种子，逐部件全字段比对）。

### 其他查找工具

- `get_mineshafts(mc, seed, cx0, cz0, cx1, cz1, out)`：扫描区块矩形（含边界）内的废弃矿井，`out` 为 `Option<&mut [Pos]>`，返回总数（可能超过写入数）。`Mineshaft` 不走 region 网格，不要对它用 `get_structure_pos` 的 region 扫描思路（`get_structure_pos(Mineshaft, ...)` 只查单区块）。
- `get_end_islands(mc, seed, chunk_x, chunk_z)`：末地小岛（0–2 个，含半径）。
- `move_structure(base_seed, dreg_x, dreg_z)`：把结构基准种子平移若干 region（48 位种子搜索用）。
- `get_population_seed` / `chunk_generate_rnd` / `get_shadow`：装饰/部件级种子助手。

## 四连底座高速搜索（quadbase）

找种工具核心：在 48 位种子空间中搜索「四连底座」——相邻四个 region `(0,0)/(0,1)/(1,0)/(1,1)` 中的四个同种结构（典型：沼泽小屋、海底神殿）能被一个半径 128 方块的球包住，一个 AFK 点即可覆盖四座刷怪结构（四连女巫小屋刷怪塔）。移植自 cubiomes `quadbase.c`/`quadbase.h`，位于 `structure::quadbase`（模块文档有完整原理说明）。

- 底座判定（输入只看种子低 48 位，返回包球半径 `f32`，0 = 不合格，越小越好）：
  - `is_quad_base(&sconf, seed, radius)`：按结构类型分派的总入口（小屋 radius=128 走优化的 `is_quad_base_feature24`，其余 radius 走一般路径 `is_quad_base_feature`；海底神殿走 `is_quad_base_large`；未支持的类型 panic，对应 C 的 `exit(-1)`）。
  - `is_quad_base_feature24_classic`：只认经典星座的特化变体，命中恒返回 1.0。
  - 返回值与 C 的 `float` 逐位一致（`f32::sqrt` ↔ `sqrtf`，C 侧以 `-ffp-contract=off` 编译）。
- 低 20 位星座表：`LOW20_QUAD_IDEAL` / `LOW20_QUAD_CLASSIC` / `LOW20_QUAD_HUT_NORMAL` / `LOW20_QUAD_HUT_BARELY`（与 C 静态表一致但**不含**结尾的 0），`get_quad_hut_cst(low20)` 做分类。表值已含盐：实际底座种子的低 20 位是 `(表值 - salt) & 0xfffff`。
- 扫描器：
  - `scan_for_quads(&sconf, radius, s48, low_bits, low_bit_n, salt, x, z, w, h, qplist)`：在 region 坐标矩形（含边界，w/h 为偏移量）内找 `s48` 的四连结构，只检查低比特属于 `low_bits` 的变换底座，命中写入 `qplist` 并返回数量。小屋用 `low_bit_n=20` + 星座表；其他结构可用 `low_bit_n=48` 指定单个底座。
  - `search_all48(threads, low_bits, check, stop)`：多线程搜索全部 48 位种子，`low_bits = Some((值集, 位数))` 时只枚举低比特匹配的子集（完整搜四连小屋：`Some((LOW20_QUAD_IDEAL 减 salt 后的值集, 20))`，约 8 亿次判定）；`stop` 为 `Option<&AtomicBool>`。与 C 的差异：不支持输出文件与断点续传；结果顺序与 C 一致（线程分区 × 高位块步进 × 值集数组序，**非**全局升序）。
- `get_optimal_afk(p, ax, ay, az)`：求四个结构（位置 `p: [Pos; 4]`，尺寸 ax×ay×az）的最佳 AFK 站位，返回 `(站位, 平面刷怪面积)`。

```rust
use minecraft_seed_core::McVersion;
use minecraft_seed_core::structure::{
    LOW20_QUAD_HUT_NORMAL, StructureType, get_config, is_quad_base, scan_for_quads, Pos,
};

let conf = get_config(StructureType::SwampHut, McVersion::V1_13).unwrap();
// 判定单个 48 位底座（高 16 位任意，不影响结构位置）
if is_quad_base(&conf, seed, 128) != 0.0 { /* 四连小屋底座 */ }

// 已知底座种子，扫描它附近 region 里全部四连小屋的 region 坐标
let mut qp = [Pos::default(); 64];
let n = scan_for_quads(&conf, 128, seed, LOW20_QUAD_HUT_NORMAL, 20,
    conf.salt as u32 as u64, -8, -8, 16, 16, &mut qp);
```

- golden 验证见 `tests/bundle_d_golden.rs`：窗口扫描候选列表（小屋 1.13+/1.12-、radius 124 一般路径、经典星座、海底神殿 radius 128/200）与 `search_all48` 全量 74474 个结果的数量/校验和/首尾样本均与 C 一致。
- 注意：四连底座只保证**位置**成簇；实际刷怪还需各结构群系可行（`is_viable_structure_pos`）且海底神殿另需水域群系判定。

## 要塞 `StrongholdIter`

```rust
use minecraft_seed_core::structure::{StrongholdIter, init_first_stronghold};

// 只要近似位置（±112 方块，只看种子低 48 位，无需群系）：
let approx = init_first_stronghold(mc, seed);

// 精确位置：迭代器
let mut it = StrongholdIter::new(mc, seed);
loop {
    let remaining = it.next(Some(&g));  // 返回这座之后还剩多少座
    let exact = it.pos;                 // 精确位置（楼梯间在区块内 (4,4)）
    let next_approx = it.nextapprox;    // 下一座近似位置
    if remaining <= 1 { break; }        // 1.9+ 共 128 座，1.8- 共 3 座
}
```

- B1.8–1.19.2：`next(Some(&g))` 必须传主世界 Generator（`locate_biome` 群系检查）；1.19.3+ 可传 `None` 只迭代近似位置。B1.7- 没有要塞：`next` 直接返回 0（不消耗随机数），`init_first_stronghold` 仍可给出首座近似位置。
- `is_stronghold_biome(mc, id)`：单群系是否可生成要塞（含 MC-199298 模拟）。

## 出生点与史莱姆区块

```rust
use minecraft_seed_core::structure::{estimate_spawn, get_spawn, is_slime_chunk};

let spawn = get_spawn(&g);           // Pos（方块坐标，精确出生点），g 为主世界生成器
let est = estimate_spawn(&g);        // 近似出生点（不含地形修正），计算更便宜
let slime = is_slime_chunk(seed, x >> 4, z >> 4); // 区块坐标，与版本无关
```

- `get_spawn` 对应 cubiomes `getSpawn`：`estimate_spawn` 结果再经地表高度修正（≤1.12 随机抖动 + grass 群系判定，1.13–1.17 螺旋 4×4 区块扫描，1.18+ ±5 区块螺旋；B1.7- 直接返回 `(0, 0)`）。与 mcseedmap.com 显示的出生点**逐位一致**（见下文「与 mcseedmap.com 的端到端一致性验证」）；注意其警告同样适用：出生点依赖 grass 方块，可能与实际世界略有出入。
- `estimate_spawn` 是**近似**出生点：B1.8–1.17 在 ±256 方块内伪随机选取可行群系位置（找不到退回 `(8, 8)`）；1.18+ 做气候参数适应度搜索（`findFittestPos`）；B1.7- 恒为 `(0, 0)`（与 C 一致）。只是 `get_spawn` 的第一阶段，不需要精确值时更便宜。
- `is_slime_chunk` 只依赖种子与区块坐标，纯 Java 版规则。

## 错误与边界语义汇总

| 情况 | 行为 |
| --- | --- |
| `get_config`：版本不支持该结构 | 返回 `None` |
| `get_structure_pos`：该 region 不生成（稀有度等） | 返回 `None` |
| `get_variant`：类型不支持 / 实例不可生成 | 返回 `None` |
| `is_viable_structure_pos`：不可行 | 返回 `0`；C 会 `exit(1)` 的未知类型组合这里返回 `0` |
| `BiomeId::from_i32`：未知 ID | 返回 `Option::None`（`-1` 除外，映射为 `BiomeId::None`） |
| `is_viable_feature_biome`：C 中 `exit(1)` 的类型（`Feature`/`Geode`/`EndIsland`） | 返回 `false` |
| `locate_biome` 找不到可行群系 | 返回搜索原点（与 C 一致） |
| `gen_biomes` 未 `with_seed` / 非法 scale / 末地 scale 1 | panic（见上） |
| 下界 <1.16.1、末地 <1.9 | 填充默认群系，不报错 |

## 性能建议

- **永远优先 `gen_biomes` 区域生成，不要逐点循环 `get_biome`**。分层路径（B1.8–1.17）的区域生成有层缓存复用；逐点调用会反复重算整条层链，慢一到两个数量级。
- **scale 按需选最大**：做地图预览用 scale 16 或 64 足够；结构 viability 内部自己会选正确层，不要为它预生成 scale 1 数据。scale 1（voronoi）最贵，只在需要方块级边界时用。
- 1.18+ 单点采样内部有 `dat` 缓存，`locate_biome`/`are_biomes_viable` 已利用；自己循环 `sample_biome_noise` 时才需要关心。
- `Generator` 构建成本不低（1.18+ 要建 spline 表，B1.8–1.17 要建层栈）：批量换种子时复用同一个 `Generator` 反复 `with_seed`，不要每次 `Generator::new`。
- 结构候选位置（`get_structure_pos`）是纯算术，极其廉价；扫描时先按 region 枚举候选、再对候选做 `is_viable_structure_pos`，避免对无效 region 启动群系计算。
- 多线程找种：`Generator` 不可跨线程共享，每个线程构建自己的实例；结构候选阶段（不碰群系）可自由并行。

## 与 mcseedmap.com 的端到端一致性验证

除基于本地编译 C 参考的 golden 单测外，`tests/web_consistency.rs` 直接用
**网站真实后端的输出**做端到端对拍。

### 数据来源与重新生成

- 引擎：`reference/site/api.wasm`（mcseedmap.com 实际部署的 cubiomes WASM，
  版本号 `v=26.3-terrain-3`）。
- 导出脚本：`node reference/site/dump_golden.mjs`。脚本绕过 emscripten 胶水
  （Node 下会挂起），手动实例化 WASM 并调用其导出函数（`generate_area` /
  `find_spawn` / `find_strongholds` / `get_structure_in_regions`），覆盖
  **10 个版本 × 5 个种子 = 50 组用例**。
- 产物（脚本同时刷新两个文件）：
  - `tests/fixtures/web_golden.json`：原始 JSON，供人工查看/外部工具用。
  - `tests/web_golden_data.rs`：Rust 静态数组（`pub static CASES: &[WebCase]`），
    测试通过 `#[path]` 引入，保持 crate 运行时零依赖（测试里不解析 JSON）。

### 覆盖范围与结果（50 用例全绿）

| 项目 | 比较方式 | 结果 |
| --- | --- | --- |
| 要塞 | 前 10 座逐一**精确相等**（1.8- 仅 3 座，网站以 -1 填充） | 50/50 全对 |
| 群系区域 | 64×64 @ scale 4、起点 (-128,-128)，4096 个 id **逐一精确相等** | 50/50 全对 |
| 结构（11 种） | [-8,8)² region 网格内 `get_structure_pos` + `is_viable_structure_pos`，与网站列表**集合相等** | 50/50 × 11 全对 |
| 出生点 | `get_spawn` vs 网站 `find_spawn`（= cubiomes `getSpawn`）**精确相等** | 50/50 全对 |

### 已核实的语义差异 / 对齐要点

- **出生点**：网站的 `find_spawn` 是 cubiomes 的 **`getSpawn`**
  （`estimateSpawn` + 地表高度地形修正），不是 `estimateSpawn`。已用
  `reference/gen/spawncheck.c`（clang 编译本地 cubiomes，输出两个函数的
  50 组结果）核实：网站输出与 C 的 `getSpawn` 50/50 完全一致。本库的
  `get_spawn` 已移植完整 `getSpawn` 管线（含 `SurfaceNoise` 与
  `mapApproxHeight` 地表高度近似），与网站输出 50/50 **精确相等**；
  `estimate_spawn` 仍保留为不含地形修正的廉价近似（与 `getSpawn` 的实测
  最大切比雪夫偏差 48 方块）。
- **generate_area 坐标**：x/z 是 **1:4 群系比例坐标**（与 cubiomes `Range`
  一致，不做 ÷4 换算）；y 在 scale≠1 时为 1:4 垂直单位，网站默认
  yHeight=320 → 本库传 `y = 320/4 = 80`。实测该区域 y=80 与 y=320 输出
  完全相同（均在地表之上，无洞穴群系参与）。
- **get_structure_in_regions(range=8)**：覆盖以原点为中心的
  **[-8, 8) × [-8, 8) region 网格**（16×16，range 是半径而非边长）。
  该结论由网站输出坐标 ÷(regionSize×16) 反推并经全量对拍验证。
- **mineshaft / treasure**：网站此接口按同样的 region 网格枚举（这两种
  结构 regionSize=1，即逐区块），输出稀疏但非空，与本库逐区块计算结果
  集合相等。

## 未覆盖清单（明确不做/未做）

以下逐项以源码为准；对接时**不要**依赖：

1. ~~Bedrock 带群系过滤版结构定位~~（**已实现**）：`bedrock::structures_in_regions_filtered` 完整移植了 wasm 的 `be_get_filtered_structures_in_regions`（func21），含 54 层 Bedrock 群系层栈（`bedrock::layers::LayerStack`）与 9 种结构的过滤规则（村庄/沙漠神殿/女巫小屋/丛林神庙/雪屋/海底神殿双段/林地府邸/埋藏的宝藏/掠夺者前哨站），与网站 wasm 91 用例 × 15 类型逐一对拍（`tests/bedrock_filtered_consistency.rs`）。注意 mcseedmap.com 自身未启用此版（其 bedrock-worker.js 注释说明底图复用 Java 引擎），Bedrock 群系底图直接用本库 Java 版 `Generator` 即可。`McVersion` 均为 Java 版本号，Bedrock 版本用 `bedrock::BedrockVersion`。
2. ~~精确出生点 `getSpawn`~~（已移植：`structure::get_spawn`，依赖的
   `SurfaceNoise` / `Generator::map_approx_height` 地表高度管线已一并移植，
   见 `tests/bundle_b_golden.rs`）。
3. ~~地形级 viability~~（已移植：`is_viable_structure_terrain` /
   `is_viable_end_city_terrain` / `is_end_chunk_empty`，以及折跃门
   `get_linked_gateway_chunk` / `get_linked_gateway_pos` / 小岛
   `map_end_island_height`）。注意 cubiomes 的 `isViableStructureTerrain`
   本身只做 depth 气候参数近似（注释明示 "subject to change"），并非真实
   地表高度判定。
4. ~~**结构部件生成**~~（已移植：`structure::pieces` 模块的
   `get_end_city_pieces` / `get_fortress_pieces` / `get_house_list`，
   见 `tests/bundle_c_golden.rs` 与「结构部件生成」小节）。
   `get_variant` 只给朝向/起始部件/包围盒。
5. **末地 scale 1**：`genEndScaled` 的 1:1 voronoi 平面缩放（`mapVoronoi114`/`mapVoronoiPlane` 的末地路径）未移植，`gen_biomes` 在末地 `scale == 1` 时 panic。末地其他 scale（4/16/64/…）正常。
6. ~~**`quadbase.c`**~~（已移植：`structure::quadbase` 模块，四连小屋/海底神殿
   底座判定、`scan_for_quads`、`search_all48`、`get_optimal_afk`，见
   「四连底座高速搜索（quadbase）」小节与 `tests/bundle_d_golden.rs`。
   未移植的仅 C 的文件输出/断点续传外壳）。
7. **`biomfilter.c`**：群系过滤器（按条件批量筛种子）未移植。可用 `gen_biomes` 区域生成自行实现。
8. ~~**Beta 1.7 及更早版本**~~（已移植：`McVersion` 下界扩展为 `B1_7`
   （对齐 cubiomes 的 `MC_B1_7`），主世界群系走 `noise::beta` 气候噪声 +
   海洋判定，`map_approx_height` 有 beta 分支，出生点/要塞/结构配置的
   老版本行为已对齐，golden 见 `tests/bundle_e_golden.rs`）。残余限制：
   B1.7- 的 `is_viable_structure_pos` 不可用（C 同样只做了半成品）；
   `samplePerlinBeta17Terrain` 在 cubiomes 中对置换表越界读（UB），本库按
   MC Beta 原版的 512 项对折表语义（`& 0xff`）移植，golden 由修正版
   `reference/gen/noise_beta17_masked.c` 生成；Alpha 1.1 及更早不支持。
9. **下界/末地 1.16.1-/1.9- 的真实群系**：与 cubiomes 一致，分别填充 `nether_wastes`/`the_end`，并非历史版本的真实世界生成。
10. **1.13 前 `Feature` 统一类型的细分**：1.12 及更早的沙漠神殿/丛林神庙/沼泽小屋/雪屋共用 `Feature` 生成尝试，位置由 `Feature` 或各类型配置算出（salt 相同），细分判定由 `is_viable_structure_pos` 的群系检查完成——与 cubiomes 行为一致，但 `is_viable_feature_biome(Feature, ...)` 恒返回 `false`（C 中为 `exit(1)`）。
