# minecraft_seed_core

纯 Rust、零外部依赖的 Minecraft 种子计算核心库（**Java 版** + **Bedrock 版**）。

本项目将 [cubiomes](https://github.com/Cubitect/cubiomes) —— [mcseedmap.com](https://mcseedmap.com) 的 WebAssembly 后端所使用的同一套 C 算法库 —— 逐函数移植为纯 Rust，并用 C 参考实现生成的 golden 向量做逐位验证。因此在本库覆盖的功能范围内，计算结果与 mcseedmap.com Web 端一致。Bedrock 版走网站的另一套引擎（`bedrock.wasm`，MT19937 体系），已同样逐指令移植（`bedrock` 模块），见下文「Bedrock 版支持」。

## 功能矩阵

| 功能 | 支持版本 | 说明 |
| --- | --- | --- |
| 主世界群系 | Beta 1.7 – 1.21.4 | Beta 1.7- 气候噪声（温度/湿度 64×64 表）+ 地形列噪声海洋判定；B1.8–1.17 分层 LayerStack；1.18+ 多噪声 + 群系搜索树 |
| 下界群系 | 1.16.1 – 1.21.4 | 多噪声；更早版本按 cubiomes 行为填充 `nether_wastes` |
| 末地群系 | 1.9 – 1.21.4 | simplex 高地噪声；更早版本填充 `the_end` |
| 群系 scale 1（1:1） | 主世界全部支持版本、下界 1.16.1+ | B1.7- 由 beta 噪声路径直接生成（支持任意 2 的幂 scale）；B1.8–1.17 走 voronoi 缩放；末地 scale 1 已支持 |
| large biomes 世界类型 | 主世界 B1.8 及以后 | `Generator::with_large_biomes(true)`（B1.7- 无此世界类型） |
| 结构候选定位 `get_structure_pos` | 按结构类型见 `get_config` | 25 种 `StructureType` 的 spacing/separation/salt 配置表全版本快照验证 |
| 结构群系可行性 `is_viable_structure_pos` | B1.8 及以后 | 含 B1.8–1.17 的粗层剪枝模拟与 1.18+ 的变体采样点；B1.7- 不可用（C 同样只做了一半，调用会 panic） |
| 结构变体 `get_variant` | 部分类型 | 村庄、堡垒、远古城市、废弃传送门、神殿/神庙/沼泽小屋、雪屋、紫晶洞、试炼密室、海底神殿 |
| 结构部件生成 `get_end_city_pieces` / `get_fortress_pieces` / `get_house_list` | 末地城 1.9+、堡垒全版本、村庄 1.13- | 逐部件输出类型/位置/包围盒/朝向/depth；堡垒自动区分 ≤1.15 与 1.16+ 随机源路径 |
| 要塞 `StrongholdIter` | B1.8 及以后 | B1.8–1.8 共 3 座，1.9+ 128 座环带；B1.7- 没有要塞（`next` 直接返回 0） |
| 精确出生点 `get_spawn` | 全支持版本 | 对应 cubiomes `getSpawn`（`estimateSpawn` + 地表地形修正），与 mcseedmap 显示值逐位一致；B1.7- 恒为 `(0, 0)` |
| 出生点估计 `estimate_spawn` | 全支持版本 | 近似出生点（`get_spawn` 的第一阶段，不含地形修正），更便宜；B1.7- 恒为 `(0, 0)` |
| 地表高度近似 `Generator::map_approx_height` | 主世界全支持版本、末地 1.9+ | 1:4 比例 `ApproxHeight`（高度 + 群系 ID）；1.18+ 走 depth 气候参数，B1.8–1.17 走核加权割线法，B1.7- 走 `approxSurfaceBeta` 列噪声插值（不输出群系 ID），末地转发 `mapEndSurfaceHeight` |
| 地形级 viability `is_viable_structure_terrain` / `is_viable_end_city_terrain` / `is_end_chunk_empty` | 1.18+ 主世界 / 末地 1.9+ | 沙漠神殿/丛林神庙/林地府邸四角 depth 判定；末地城最小地表高度；末地空 chunk 判定 |
| 末地折跃门 `get_linked_gateway_chunk` / `get_linked_gateway_pos` | 1.13+ | 含 1.17+ MC 原版落点 bug 的逐位复刻 |
| 四连底座高速搜索 `structure::quadbase` | 按结构类型见 `get_config` | 四连小屋/海底神殿等连体式底座判定（`is_quad_base*`）、region 扫描（`scan_for_quads`）、全 48 位多线程找种（`search_all48`）、AFK 站位（`get_optimal_afk`）；C 的文件断点续传外壳未移植 |
| 史莱姆区块 `is_slime_chunk` | 全支持版本 | Java 版规则，与版本无关 |
| 废弃矿井 `get_mineshafts` | 全支持版本 | 含 1.13- 的距离衰减规则 |
| 种子搜索 `search::find_biomes` / `find_structures` / `find_biomes_with_structure` | 1.7 – 1.21.4 | 与网站 find_biomes/find_structures 语义逐一精确一致（含 48+16 位打包返回值） |
| **Bedrock** 结构散布 `bedrock::structures_in_regions` / `find_structures` | 1.16.0 – 26.50 | 20 种 `BeStructureType`，region 网格 + MT19937 偏移，与网站 wasm 逐点一致 |
| **Bedrock** 出生点 `bedrock::get_spawn` / 要塞 `bedrock::get_strongholds` | 与版本无关 | 只用种子低 32 位；要塞角度含 wasm 定制的 musl 变体 sin/cos |

版本枚举为 `McVersion::B1_7` … `McVersion::V1_21`（含 `B1_8`、`V1_0` … `V1_6` 与 `V1_16_1`、`V1_19_2`、`V1_21_1`、`V1_21_3` 等细分项，对齐 cubiomes 的 `MCVersion`），`McVersion::name()` 给出如 `"b1.7.3"`、`"1.18.2"` 的字符串。

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
- `cargo run --example versions_demo`：全版本遍历同一种子的群系差异 + large biomes。
- `cargo run --example dimensions`：下界/末地群系 + 折跃门定位。
- `cargo run --example heightmap_spawn`：近似地表高度图 + 精确/估计出生点对比。
- `cargo run --example climate_noise`：1.18+ 气候多噪声参数采样。
- `cargo run --example structure_pieces`：末地城/堡垒部件树、旧版村庄房屋列表、结构变体。
- `cargo run --example terrain_viability`：群系可行性与地形级可行性对比。
- `cargo run --example quad_base --release`：四连女巫小屋底座判定与 region 扫描。
- `cargo run --example bedrock_demo`：Bedrock 结构散布（含过滤版）、出生点、要塞。
- `cargo run --example seed_search --release`：三种种子搜索 API（与网站语义一致）。
- `cargo run --example perf_legacy --release`：旧版分层群系源的区域生成性能冒烟。

详细对接文档（坐标语义、错误/边界语义、性能建议、未覆盖清单）见 [docs/INTEGRATION.md](docs/INTEGRATION.md)。

## 测试

```sh
cargo test          # 60+ 项 golden 向量测试，与 C 参考实现逐位比较
cargo clippy --all-targets -- -D warnings
cargo build --examples
```

golden 向量由 `reference/gen/` 下的 C 程序（`layervec.c`、`biomevec.c`、`structvec.c`、`noisevec.c`、`xorovec.c`、`jvec.c`、`bundleavec.c`、`bundlebvec.c`、`bundlecvec.c`、`bundledvec.c`、`bundleevec.c` 等，直接编译自 cubiomes 源码）生成；`gen_*_tests.py` 脚本把输出转为 Rust 测试断言。测试覆盖：RNG（Java LCG / Xoroshiro）、噪声（Perlin/Octave/DoublePerlin/SurfaceNoise/Beta 气候与地形噪声）、Beta 1.7–1.21 全版本 × 全群系区域快照、全版本 × 全结构类型的配置与位置、要塞/出生点/史莱姆区块、地表高度近似与地形级 viability/末地折跃门、末地城/下界堡垒/村庄的结构部件生成、四连底座高速搜索（窗口扫描候选列表与全 48 位搜索摘要）等。

注意 Beta 1.7 的 golden 是个特例：cubiomes 的 `samplePerlinBeta17Terrain` 对 257 字节置换表存在越界读（UB），本库按 MC Beta 原版的 512 项对折表语义（下标 `& 0xff`）移植；`bundleevec.c` 因此链接修正后的 `noise_beta17_masked.c` 而非原版 `noise.c`（验证见 `reference/gen/betacheck.c`）。

## 与 mcseedmap.com 的端到端验证

`tests/web_consistency.rs` 用**网站真实的 WASM 引擎**（`mcseedmap.com/workers/api.wasm`，即 cubiomes 的 Emscripten 编译产物）导出的输出做对拍：10 个版本 × 5 个种子的要塞坐标、64×64 群系区域（4096 个 id）、11 种结构的可行位置、出生点（网站 `find_spawn` = cubiomes `getSpawn`，本库 `get_spawn`）全部**逐一精确相等**。重新生成 golden 的方法：`node reference/site/dump_golden.mjs`（需要先从网站下载最新的 `api.wasm`，详见 docs/INTEGRATION.md「与 mcseedmap.com 的端到端一致性验证」一节）。

`tests/bedrock_consistency.rs` 用网站的 Bedrock 引擎（`workers/bedrock.wasm`）做对拍：13 个版本 × 7 个种子的出生点、3 座要塞、15 种结构的 region 散布列表、全部配置表快照与 MT19937 原始向量全部**逐一精确相等**。重新生成 golden：`node reference/site/dump_bedrock_golden.mjs`。

## Bedrock 版支持

Bedrock 模块（`minecraft_seed_core::bedrock`）与 Java 版是两套独立算法：

- 随机源为标准 **MT19937**，且出生点/要塞只用种子**低 32 位**（与版本无关）；结构散布的 region 种子用完整 64 位种子 + salt + region 坐标线性组合（wrapping i64），MT 初始化取其低 32 位；
- 版本分派只体现在结构配置表：village / ocean_ruin / shipwreck 在 1.18+（mc>17）换用新配置；
- 版本枚举为 `BedrockVersion::V1_16_0` … `V26_50`（对应网站 wasm 的 mcVersion 14…28），`BedrockVersion::name()` 给出 `"1.21.50"` 等字符串；
- `bedrock::get_config` 给出 spacing/separation/salt/mt_count 配置；`structures_in_regions`（以原点为中心 ±range region）与 `find_structures`（以任意方块坐标为中心）返回全部候选位置；`get_spawn` / `get_strongholds` 与网站输出逐位一致（要塞的 sin/cos 是 wasm 内嵌 musl 变体的逐指令移植，见 `src/bedrock/trig.rs`）。

另提供 `bedrock::structures_in_regions_filtered`（wasm `be_get_filtered_structures_in_regions` 的完整移植：54 层 Bedrock 群系层栈 + 9 种结构的过滤规则，与网站 wasm 对拍全绿；网站自身未启用此版）。Bedrock 侧**不存在**独立群系生成器（网站引擎中本就没有，底图复用 Java 引擎）。

## 已知限制

以下功能 cubiomes 有而本库**未移植**（对接时请勿依赖）：

- ~~Bedrock 带群系过滤的结构定位~~：已实现（`bedrock::structures_in_regions_filtered`，含 54 层群系层栈；网站自身未启用此版，仅为算法完整性移植）。
- 末地群系的 scale 1（1:1 voronoi 平面缩放）：调用会 panic。
- ~~`biomfilter.c`~~：cubiomes master 本身没有此文件（属 cubiomes-viewer）；已由 `search` 模块覆盖网站实际提供的种子搜索能力（find_biomes / find_structures / find_biomes_with_structure，与网站 api.wasm 对拍一致）。
- Alpha 1.1 及更早版本（`McVersion` 下界为 Beta 1.7，对齐 cubiomes 的 `MC_B1_7`）。

此外注意：`Generator::gen_biomes` 在未调用 `with_seed`、B1.8–1.17 主世界 `scale` 不是 1/4/16/64/256（B1.7- 接受任意 2 的幂）、或末地 `scale == 1` 时会 panic；`get_config` 对版本不支持的结构返回 `None`；`get_structure_pos` 对该 region 不生成的情况返回 `None`。

## License

MIT（与 cubiomes 一致）。
