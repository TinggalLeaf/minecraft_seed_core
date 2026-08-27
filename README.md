# minecraft_seed_core

纯 Rust、零外部依赖的 Minecraft **Java 版**种子计算核心库。

本项目将 [cubiomes](https://github.com/Cubitect/cubiomes) —— [mcseedmap.com](https://mcseedmap.com) 的 WebAssembly 后端所使用的同一套 C 算法库 —— 逐函数移植为纯 Rust，并用 C 参考实现生成的 golden 向量做逐位验证。因此在本库覆盖的功能范围内，计算结果与 mcseedmap.com Web 端一致（mcseedmap 的 Bedrock 版走另一套逻辑，本库**不支持** Bedrock，见下文「已知限制」）。

## 功能矩阵

| 功能 | 支持版本 | 说明 |
| --- | --- | --- |
| 主世界群系 | 1.7 – 1.21.4 | 1.7–1.17 分层 LayerStack；1.18+ 多噪声 + 群系搜索树 |
| 下界群系 | 1.16.1 – 1.21.4 | 多噪声；更早版本按 cubiomes 行为填充 `nether_wastes` |
| 末地群系 | 1.9 – 1.21.4 | simplex 高地噪声；更早版本填充 `the_end` |
| 群系 scale 1（1:1 voronoi） | 主世界全部支持版本、下界 1.16.1+ | 末地 scale 1 未移植（调用会 panic） |
| large biomes 世界类型 | 主世界全部支持版本 | `Generator::with_large_biomes(true)` |
| 结构候选定位 `get_structure_pos` | 按结构类型见 `get_config` | 25 种 `StructureType` 的 spacing/separation/salt 配置表全版本快照验证 |
| 结构群系可行性 `is_viable_structure_pos` | 全支持版本 | 含 1.7–1.17 的粗层剪枝模拟与 1.18+ 的变体采样点 |
| 结构变体 `get_variant` | 部分类型 | 村庄、堡垒、远古城市、废弃传送门、神殿/神庙/沼泽小屋、雪屋、紫晶洞、试炼密室、海底神殿 |
| 要塞 `StrongholdIter` | 全支持版本 | 1.8 及以前 3 座，1.9+ 128 座环带 |
| 出生点估计 `estimate_spawn` | 全支持版本 | 近似出生点；mcseedmap 显示的是 cubiomes `getSpawn`（含地表地形修正），与 `estimate_spawn` 最多差几十格，见下文「端到端验证」 |
| 史莱姆区块 `is_slime_chunk` | 全支持版本 | Java 版规则，与版本无关 |
| 废弃矿井 `get_mineshafts` | 全支持版本 | 含 1.13- 的距离衰减规则 |

版本枚举为 `McVersion::V1_7` … `McVersion::V1_21`（含 `V1_16_1`、`V1_19_2`、`V1_21_1`、`V1_21_3` 等细分项，对齐 cubiomes 的 `MCVersion`），`McVersion::name()` 给出如 `"1.18.2"` 的字符串。

## 快速开始

本库尚未发布到 crates.io，以 git 依赖使用：

```toml
[dependencies]
minecraft_seed_core = { git = "https://<你的仓库地址>" }
```

查一个方块位置的群系（坐标为 1:4 群系比例，即方块坐标除以 4）：

```rust
use minecraft_seed_core::{Dimension, Generator, McVersion};

let g = Generator::new(McVersion::V1_20).with_seed(Dimension::Overworld, 12345);
let biome = g.get_biome(0, 319 >> 2, 0); // 原点附近、地表高度的群系
println!("{biome:?}");
```

批量生成一块群系区域（远比逐点快）：

```rust
use minecraft_seed_core::generator::Range;

// 64x64 个 1:4 单元 = 256x256 方块；输出索引为 out[i_y*sx*sz + i_z*sx + i_x]
let area = g.gen_biomes(Range::new(4, -32, -32, 64, 64));
```

找结构（村庄）：

```rust
use minecraft_seed_core::structure::{get_structure_pos, is_viable_structure_pos};
use minecraft_seed_core::StructureType;

// 候选位置只依赖种子低 48 位与 region 坐标
if let Some(pos) = get_structure_pos(StructureType::Village, McVersion::V1_20, 12345, 0, 0) {
    // 群系可行性检查（需要已按主世界初始化的 Generator）
    if is_viable_structure_pos(StructureType::Village, &g, pos.x, pos.z, 0) != 0 {
        println!("village at ({}, {})", pos.x, pos.z);
    }
}
```

更多完整示例见 `examples/`：

- `cargo run --example seed_info`：出生点、要塞、出生点附近的史莱姆区块。
- `cargo run --example find_structures`：扫描范围内某结构的候选位置并做群系可行性验证。
- `cargo run --example biome_map`：生成群系区域并打印 ASCII 图。
- `cargo run --example perf_legacy --release`：旧版分层群系源的区域生成性能冒烟。

详细对接文档（坐标语义、错误/边界语义、性能建议、未覆盖清单）见 [docs/INTEGRATION.md](docs/INTEGRATION.md)。

## 测试

```sh
cargo test          # 60+ 项 golden 向量测试，与 C 参考实现逐位比较
cargo clippy --all-targets -- -D warnings
cargo build --examples
```

golden 向量由 `reference/gen/` 下的 C 程序（`layervec.c`、`biomevec.c`、`structvec.c`、`noisevec.c`、`xorovec.c`、`jvec.c` 等，直接编译自 cubiomes 源码）生成；`gen_*_tests.py` 脚本把输出转为 Rust 测试断言。测试覆盖：RNG（Java LCG / Xoroshiro）、噪声（Perlin/Octave/DoublePerlin）、1.7–1.21 全版本 × 全群系区域快照、全版本 × 全结构类型的配置与位置、要塞/出生点/史莱姆区块等。

## 与 mcseedmap.com 的端到端验证

`tests/web_consistency.rs` 用**网站真实的 WASM 引擎**（`mcseedmap.com/workers/api.wasm`，即 cubiomes 的 Emscripten 编译产物）导出的输出做对拍：10 个版本 × 5 个种子的要塞坐标、64×64 群系区域（4096 个 id）、11 种结构的可行位置全部**逐一精确相等**；出生点在容差内（网站用 `getSpawn` 含地形修正，本库为 `estimate_spawn`）。重新生成 golden 的方法：`node reference/site/dump_golden.mjs`（需要先从网站下载最新的 `api.wasm`，详见 docs/INTEGRATION.md「与 mcseedmap.com 的端到端一致性验证」一节）。

## 已知限制

以下功能 cubiomes 有而本库**未移植**（对接时请勿依赖）：

- **Bedrock 版**：mcseedmap 的 Bedrock 使用另一套算法，本库只覆盖 Java 版。
- `getSpawn` 精确出生点（依赖地表高度近似噪声管线）；本库提供 `estimate_spawn` 近似值。
- `isViableStructureTerrain` / `isViableEndCityTerrain` 等地形级可行性检查：本库的 viability 只做群系层面判定。
- 结构部件生成：`getEndCityPieces` / `getFortressPieces` / 村庄 `getHouseList`。
- 末地群系的 scale 1（1:1 voronoi 平面缩放）：调用会 panic。
- `quadbase.c`（四连底座高速搜索）与 `biomfilter.c`（群系过滤器）—— 找种级批量工具。
- Beta 1.7 及更早版本（`betacheck` 相关逻辑仅在参考脚本中保留）。

此外注意：`Generator::gen_biomes` 在未调用 `with_seed`、旧版主世界 `scale` 不是 1/4/16/64/256、或末地 `scale == 1` 时会 panic；`get_config` 对版本不支持的结构返回 `None`；`get_structure_pos` 对该 region 不生成的情况返回 `None`。

## License

MIT（与 cubiomes 一致）。
