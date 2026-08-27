# minecraft_seed_core 对接文档

本文档面向要把本库接入自己程序（种子查找器、地图渲染器、Discord bot 等）的开发者。所有 API 名称与 `src/` 源码一致；示例代码均已验证可编译（见 `examples/`）。

## 模块地图

| 模块 | 内容 | 对应 cubiomes |
| --- | --- | --- |
| `version` | `McVersion`、`Dimension` | `MCVersion` / `Dimension` |
| `biome` | `BiomeId` 枚举与分类/海洋/突变等谓词 | `biomes.h` / `biomes.c` |
| `rng` | `JavaRandom`（48 位 LCG）、`Xoroshiro`、种子流水线（`layer_salt`/`start_seed`/`chunk_seed`） | `java.h` / `xoro.h` / `layers.c` 的种子助手 |
| `noise` | `PerlinNoise`、`OctaveNoise`、`DoublePerlinNoise`、`BiomeNoise` | `noise.c` / `biomenoise.c` |
| `generator` | `Generator`、`Range`，按版本分派（`v1_18`/`layers`/`nether`/`end`/`voronoi`） | `generator.c` / `layers.c` |
| `structure` | `StructureType`、`get_config`、`get_structure_pos`、`is_viable_structure_pos`、`get_variant`、`StrongholdIter`、`estimate_spawn`、`is_slime_chunk` 等 | `finders.c` / `finders.h` |

crate 根部 re-export：`BiomeId`、`Generator`、`Range`、`StructureType`、`Dimension`、`McVersion`。

## 版本与维度

```rust
use minecraft_seed_core::{McVersion, Dimension};

let mc = McVersion::V1_20;
assert_eq!(mc.name(), "1.20.6");           // 人类可读字符串
assert!(mc >= McVersion::V1_18);           // 枚举按发布时间排序，可直接比较
assert!(mc.has_multi_noise_biomes());      // 1.18+ 为 true
for v in McVersion::ALL { /* 19 个版本，升序 */ }
```

- `V1_X` 表示该大版本的**最新补丁**（与 cubiomes 一致），如 `V1_16` = 1.16.5；规则有差异的小版本单列：`V1_16_1`、`V1_19_2`、`V1_21_1`、`V1_21_3`。
- `McVersion::NEWEST` / `McVersion::OLDEST` 给出支持边界（当前为 1.21.4 / 1.7.10）。
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

- `Generator::new(mc)`：1.18+ 主世界在此构建群系噪声 spline 表；1.7–1.17 的分层层栈推迟到 `with_seed`（需要 large 标志）。
- `with_large_biomes(large)`：只影响主世界（对应 C `setupGenerator` 的 `LARGE_BIOMES` flag）。
- `with_seed(dim, seed)`：可重复调用更换维度/种子，复用已构建的噪声表。
- 只读访问器：`dim()` / `seed()` / `version()` / `biome_noise()`（1.18+ 主世界的噪声，调试用）。
- `Generator: Clone`（分层路径内部含 `Cell`，因此不是 `Sync`——可以移动进别的线程，但不能跨线程共享引用；多线程找种请每个线程各建一份）。

### 坐标语义（重点）

- `get_biome(x, y, z)`：**三个坐标都是 1:4 群系比例**，即方块坐标除以 4（`x_block >> 2`）。
- `gen_biomes(Range)`：`Range { scale, x, z, sx, sz, y, sy }`
  - `scale`：水平比例因子，支持 **1、4、16、64、256**。scale 4 即默认群系比例；scale 1 是方块级（走 voronoi 扰动）。
  - `x, z`：区域西北角，**按 scale 比例**计。即 scale=4 时 `x=0` 对应方块 0，scale=1 时 `x` 就是方块坐标。
  - `y, sy`：垂直位置与尺寸。**scale != 1 时垂直比例恒为 1:4**（与水平 scale 无关），即 `y = y_block >> 2`；`sy <= 0` 视为 1。`Range::new(scale, x, z, sx, sz)` 是 2D 便捷构造（`y=0, sy=1`），`.with_y(y, sy)` 设垂直范围。
  - 输出长度 `sx*sy*sz`，索引 `out[i_y*sx*sz + i_z*sx + i_x]`。
- **y 的作用**：仅 1.18+ 主世界的群系随 y 变化（洞穴群系等）。1.7–1.17 主世界完全忽略 y（生成 2D 平面后沿 y 复制）；下界群系实际上也不随 y 变化。查地表群系的惯例采样高度是 `y = 319 >> 2`（与 `is_viable_structure_pos` 对地表结构的采样一致）；查海底神殿等用海床附近（`y = 36 >> 2`）。

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

- `Dimension::Overworld`：全版本支持（1.7–1.21）。
- `Dimension::Nether`：1.16.1+ 真实多噪声；**1.15 及更早**不报错，整个区域填充 `BiomeId::NetherWastes`（与 cubiomes 行为一致）。scale 支持 1/4/16/64/256（`scale <= 0` 视为 4）。
- `Dimension::End`：1.9+ simplex 高地噪声；**1.8 及更早**填充 `BiomeId::TheEnd`。scale 支持 4/16/64/256 及更大；**scale 1 未移植，调用 panic**。

### Panics（调用方需要避免的输入）

- 未调用 `with_seed` 就 `gen_biomes` / `get_biome`。
- 1.7–1.17 主世界 `scale` 不是 1/4/16/64/256。
- 末地 `scale == 1`。
- `StrongholdIter::next` 在 1.7–1.19.2 传 `None`（需要主世界生成器做群系检查）。

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
- 1.7–1.17 主世界的 viability 带**粗层剪枝**模拟（C 的 `mapViableBiome`/`mapViableShore`），结果与 C 逐位一致，包括那些"细层有目标群系但粗层没有而判不可行"的边角情形。

### 变体判定 `get_variant`

```rust
let sv = get_variant(StructureType::Village, mc, seed, pos.x, pos.z, biome_id);
```

- `x, z`：结构候选的方块坐标（`get_structure_pos` 的输出）。
- `biome_id`：群系变体提示（村庄必填，通常用 `is_viable_structure_pos` 的返回值）；其他类型传 `-1`。
- 返回 `Option<StructureVariant>`：不支持该类型 / 不可生成（如紫晶洞稀有度未过）时返回 `None`。
- 支持的类型：`Village`（含僵尸村庄 `abandoned`、朝向 `rotation`、包围盒）、`Bastion`（4 种 `start`）、`AncientCity`、`RuinedPortal`/`RuinedPortalN`（`giant`/`underground`/`airpocket`）、`DesertPyramid`/`JungleTemple`/`SwampHut`（1.20+ 含朝向）、`Igloo`（`basement`/`size`）、`Monument`（固定包围盒）、`Geode`（`y`/`size`/`cracked`）、`TrialChambers`。
- `rotation`：`0=0°, 1=cw90, 2=cw180, 3=cw270`；`x/z/sx/sy/sz` 为相对包围盒。

### 其他查找工具

- `get_mineshafts(mc, seed, cx0, cz0, cx1, cz1, out)`：扫描区块矩形（含边界）内的废弃矿井，`out` 为 `Option<&mut [Pos]>`，返回总数（可能超过写入数）。`Mineshaft` 不走 region 网格，不要对它用 `get_structure_pos` 的 region 扫描思路（`get_structure_pos(Mineshaft, ...)` 只查单区块）。
- `get_end_islands(mc, seed, chunk_x, chunk_z)`：末地小岛（0–2 个，含半径）。
- `move_structure(base_seed, dreg_x, dreg_z)`：把结构基准种子平移若干 region（48 位种子搜索用）。
- `get_population_seed` / `chunk_generate_rnd` / `get_shadow`：装饰/部件级种子助手。

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

- 1.7–1.19.2：`next(Some(&g))` 必须传主世界 Generator（`locate_biome` 群系检查）；1.19.3+ 可传 `None` 只迭代近似位置。
- `is_stronghold_biome(mc, id)`：单群系是否可生成要塞（含 MC-199298 模拟）。

## 出生点与史莱姆区块

```rust
use minecraft_seed_core::structure::{estimate_spawn, is_slime_chunk};

let spawn = estimate_spawn(&g);        // Pos（方块坐标），g 为主世界生成器
let slime = is_slime_chunk(seed, x >> 4, z >> 4); // 区块坐标，与版本无关
```

- `estimate_spawn` 是**近似**出生点：1.7–1.17 在 ±256 方块内伪随机选取可行群系位置（找不到退回 `(8, 8)`）；1.18+ 做气候参数适应度搜索（`findFittestPos`）。注意 mcseedmap.com 显示的是 `getSpawn`（`estimate_spawn` 结果再经地表高度修正），与本库输出的偏差在已验证的 50 组用例中 ≤48 方块（见下文「与 mcseedmap.com 的端到端一致性验证」）。精确出生点（`getSpawn`）依赖未移植的地表高度管线，不可用。
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

- **永远优先 `gen_biomes` 区域生成，不要逐点循环 `get_biome`**。分层路径（1.7–1.17）的区域生成有层缓存复用；逐点调用会反复重算整条层链，慢一到两个数量级。
- **scale 按需选最大**：做地图预览用 scale 16 或 64 足够；结构 viability 内部自己会选正确层，不要为它预生成 scale 1 数据。scale 1（voronoi）最贵，只在需要方块级边界时用。
- 1.18+ 单点采样内部有 `dat` 缓存，`locate_biome`/`are_biomes_viable` 已利用；自己循环 `sample_biome_noise` 时才需要关心。
- `Generator` 构建成本不低（1.18+ 要建 spline 表，1.7–1.17 要建层栈）：批量换种子时复用同一个 `Generator` 反复 `with_seed`，不要每次 `Generator::new`。
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
| 出生点 | `estimate_spawn` vs 网站 `find_spawn`，容差 64 方块 | 50/50 在容差内（其中 10 例精确相等） |

### 已核实的语义差异 / 对齐要点

- **出生点**：网站的 `find_spawn` 是 cubiomes 的 **`getSpawn`**
  （`estimateSpawn` + 地表高度地形修正），不是 `estimateSpawn`。已用
  `reference/gen/spawncheck.c`（clang 编译本地 cubiomes，输出两个函数的
  50 组结果）核实：网站输出与 C 的 `getSpawn` 50/50 完全一致，与
  `estimateSpawn` 仅 10/50 一致（1.7/1.12 的地形修正在这些种子上未触发）。
  本库实现的是 `estimateSpawn`，实测最大切比雪夫偏差 48 方块，故 spawn
  测试用容差 64。若后续移植 `SurfaceNoise`/`mapApproxHeight` 管线，可改为
  精确比较。
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

1. **Bedrock 版**：mcseedmap.com 的 Bedrock 走另一套独立逻辑（非 cubiomes），本库只覆盖 Java 版。所有 `McVersion` 均为 Java 版本号。
2. **精确出生点 `getSpawn`**：依赖 `SurfaceNoise` 与 `mapApproxHeight`（generator.c 的地表高度近似），噪声管线未移植。只有 `estimate_spawn` 近似值。
3. **地形级 viability**：`isViableStructureTerrain` / `isViableEndCityTerrain` 未移植。本库 viability 只做群系层面判定（及 cubiomes 自带的随机性判定），**不**保证地形上真的能生成（如海底神殿的实际海床形状）。
4. **结构部件生成**：`getEndCityPieces` / `getFortressPieces` / 村庄 `getHouseList` 未移植。`get_variant` 只给朝向/起始部件/包围盒。
5. **末地 scale 1**：`genEndScaled` 的 1:1 voronoi 平面缩放（`mapVoronoi114`/`mapVoronoiPlane` 的末地路径）未移植，`gen_biomes` 在末地 `scale == 1` 时 panic。末地其他 scale（4/16/64/…）正常。
6. **`quadbase.c`**：四连底座（quad-hut/quad-monument）高速底座搜索未移植——这是找种工具，不影响单点结构定位。
7. **`biomfilter.c`**：群系过滤器（按条件批量筛种子）未移植。可用 `gen_biomes` 区域生成自行实现。
8. **Beta 1.7 及更早版本**：`McVersion` 下界为 1.7（对齐 cubiomes 的 `MC_1_7`）。`reference/gen/betacheck.c` 等 Beta 相关逻辑仅存在于参考脚本。噪声模块虽移植了 Beta 地形噪声（`sample_beta17_terrain`），但没有对应的群系/结构入口。
9. **下界/末地 1.16.1-/1.9- 的真实群系**：与 cubiomes 一致，分别填充 `nether_wastes`/`the_end`，并非历史版本的真实世界生成。
10. **1.13 前 `Feature` 统一类型的细分**：1.12 及更早的沙漠神殿/丛林神庙/沼泽小屋/雪屋共用 `Feature` 生成尝试，位置由 `Feature` 或各类型配置算出（salt 相同），细分判定由 `is_viable_structure_pos` 的群系检查完成——与 cubiomes 行为一致，但 `is_viable_feature_biome(Feature, ...)` 恒返回 `false`（C 中为 `exit(1)`）。
