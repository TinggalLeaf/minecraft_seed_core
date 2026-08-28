# Bedrock 引擎逆向分析与移植计划

> **状态：移植已完成（2026-08）。** `src/bedrock/` 全部子模块已实现，
> `tests/bedrock_consistency.rs` 对 91 组用例 + 配置表 + MT 向量逐位对拍全绿，
> `cargo clippy --all-targets -- -D warnings` 无警告。本文档保留为逆向侦察档案。
>
> 移植期澄清的关键结论（修正下文的「待澄清」标注）：
> - `be_mt_n_get(seed: i64, n: i32)` 返回 **malloc 出的 n 个 i32 输出数组的指针**
>   （标准 MT19937，种子取低 32 位）；dump 脚本已修正，`mt_vectors` 现为真实输出序列。
> - **spawn 不做群系评估**：func24 实际只是 `MT(seed_lo)` 两次输出
>   `(mt[i] & 511) - 256`；下文第 7 节关于「spawn 需要分层群系评估」的推断不成立
>   （该分层代码存在于 wasm 中但服务于 filtered 版结构过滤，网站未使用）。
> - 要塞的 sin/cos 是 wasm 内嵌的 **musl 变体**（`__rem_pio2` 常量表被截断定制），
>   已逐指令移植到 `src/bedrock/trig.rs`；不能用 Rust std 的 sin/cos 替代。
> - **过滤版（func21）已移植**（2026-08 补充）：`bedrock::structures_in_regions_filtered`
>   + 54 层群系层栈 `bedrock::layers::LayerStack`，与 wasm 91 用例 × 15 类型对拍全绿
>   （`tests/bedrock_filtered_consistency.rs`）。移植期修复了上一版层栈移植的 4 处
>   反编译误读：f_db 的 SE 选择 `a==0` 分支（恒保持 SE，无 RNG）、f_ja 的 40/46
>   邻居判定互换、f_la 的 B_i/B_j 南邻 select 极性（S∉M2 → 保持，S∈M2 → 25/26）。
>   教训：wasm-decompile 的 `select_if(x, y, c)` 是 `c ? x : y`，br_table 标签的
>   物理顺序与逻辑索引无关，歧义处必须以 WAT 指令（wasm-objdump -d）为准。

> 本文档是 Bedrock 支持的交接文档：记录对 mcseedmap.com Bedrock 引擎
> （`bedrock.wasm`）的全部侦察结论、已解码的数据表与移植计划。
> 下次继续时按「移植计划」一节执行即可。

## 1. 站点架构侦察结论

mcseedmap.com 的计算全部在浏览器 Worker 中本地完成，两套引擎：

| | Java 版 | Bedrock 版 |
| --- | --- | --- |
| 引擎 | `workers/api.wasm`（cubiomes 的 Emscripten 编译产物，518KB） | `workers/bedrock.wasm`（自研，42KB） |
| 胶水 | `api.js` + `worker.js` + `seeder.js` | `bedrock.js` + `bedrock-worker.js` + `bedrock-seeder.js` |
| 群系地图 | `generate_area`（cubiomes genArea） | **复用 Java 引擎渲染**（bedrock-worker.js 注释明确承认：Bedrock 不做群系过滤，"biome context is shown by the map tiles"） |
| 结构/出生点/要塞 | cubiomes finders | bedrock.wasm 自有算法 |

**结论**：网站的 "Bedrock 支持" = Bedrock 规则的结构定位 + 出生点 + 要塞；
群系底图直接用 Java 引擎。因此本库的 Bedrock 模块范围与网站一致即可：
结构、出生点、要塞，不需要独立的 Bedrock 群系生成器。

## 2. 资产清单（均已下载到本仓库）

- `reference/site/bedrock.wasm`：Bedrock 引擎（v=4，42KB，无 name section，48 个函数）
- `reference/site/bedrock.js` / `bedrock-seeder.js` / `bedrock-worker.js`：胶水层
- `reference/site/bedrock.dcmp`：`wasm-decompile` 反编译输出（5595 行类 C 伪码）
- `reference/site/dump_bedrock_golden.mjs`：Node golden 导出脚本（手动实例化 wasm，绕过 emscripten 胶水）
- `tests/fixtures/bedrock_golden.json`：**已生成的 91 组 golden 用例**（13 版本 × 7 种子：
  spawn、strongholds、20 种结构的 region 列表、配置表快照）
- `reference/wabt-1.0.41/bin/`：wasm-objdump / wasm2wat 等（需要指令级细节时用）

## 3. WASM 导出/导入映射

导入 `a = { a: _exit, b: _fd_write, c: _emscripten_resize_heap }`。

| 导出 | 名称 | 签名（反编译） |
| --- | --- | --- |
| d | memory | — |
| e | ctors | `()` |
| f | `be_mt_n_get` | `(seed: i64, n: i32) -> i32`，内部是 MT19937（经典递推 `(d >> 30 ^ d) * 1812433253 + c`，见 dcmp func10）；**注意：疑似返回堆指针/数组语义待澄清** |
| g | malloc | — |
| h | （内部辅助，int_ptr 参数） | func3 |
| i | `be_int_to_float` | func52 |
| j | `be_get_structure_config` | `(structType, mcVersion) -> 静态配置表指针`（func41，br_table 分派 + `b > 17` 版本门控） |
| k | `be_find_structures` | `(mc, type, seedLo, seedHi, cx, cz, range, outCount) -> ptr`（func14） |
| l | `be_get_structures_in_regions` | `(mc, type, seedLo, seedHi, regionsRange, outCount) -> ptr`（func28） |
| m | `be_get_spawn` | `(mc, seedLo, seedHi, outX, outZ) -> void`（func24） |
| n | `be_get_strongholds` | `(mc, seedLo, seedHi, outCount) -> ptr`（func23） |
| o | `be_free` | — |
| p | `be_get_filtered_structures_in_regions` | 带群系过滤版（网站未使用，见 bedrock-worker.js 注释） |
| q/r/s/t | 小辅助函数 | func26/27 等 |

种子以 lo/hi 两个 i32 传入（`BigInt(seed) & 0xFFFFFFFF` / `>> 32`）。
返回的坐标列表为 `[x, z]` i32 对，`(-1, -1)` 为无效项。

## 4. 版本与结构类型映射

版本标签 → wasm 整数（来自页面 chunk-635 的映射表）：

```
14=1.16.0  15=1.16.220  17=1.17.40  18=1.18.0  19=1.18.30
21=1.19.0  22=1.19.80   23=1.20.0   24=1.20.80 25=1.21.0
26=1.21.50 27=26.30     28=26.50
（16、20 在表中无对应标签；26.40.27 映射到 28，26.31–26.33 映射到 27）
```

结构类型编号（网站 Bedrock 结构列表）：

```
0=Village 1=Stronghold 3=DesertTemple 4=WitchHut 5=JungleTemple 6=Igloo
7=OceanMonument 8=OceanRuin 9=Mansion 10=Shipwreck 11=RuinedPortal
12=BuriedTreasure 13=PillagerOutpost 14=NetherFortress 15=Bastion
16=EndCity 17=AncientCity 18=TrailRuin 19=TrialChamber 20=AbandonedCamp
（编号 2 在网站列表中未出现，可能是内部 Feature 占位）
```

## 5. 静态配置表（已解码）

内存 offset 1024 起，每条 16 字节 = 4×i32 `{spacing, separation, salt, flags}`，
共 15 条结构配置（之后是群系列表等其他表）：

| # | offset | spacing | sep | salt (hex) | flags | 推断（待 golden 验证） |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | 1024 | 34 | 26 | 0x9e7f70 | 4 | Village（≤1.17 档，salt=10387312 与公开资料一致） |
| 1 | 1040 | 27 | 17 | 0x9e7f70 | 4 | Village（>1.17 档，`b>17` 选择） |
| 2 | 1056 | 32 | 24 | 0xdb1471 | 2 | |
| 3 | 1072 | 32 | 27 | 0x9e7f71 | 4 | |
| 4 | 1088 | 20 | 12 | 0xdb1475 | 2 | |
| 5 | 1104 | 12 | 5 | 0xdb1475 | 4 | |
| 6 | 1120 | 80 | 60 | 0x9e7f77 | 4 | |
| 7 | 1136 | 24 | 20 | 0x9e1128f | 2 | |
| 8 | 1152 | 10 | 5 | 0x9e1128f | 4 | |
| 9 | 1168 | 40 | 25 | 0x26ac727 | 2 | |
| 10 | 1184 | 4 | 2 | 0x100fe9d | 4 | |
| 11 | 1200 | 80 | 56 | 0x9e11290 | 4 | |
| 12 | 1216 | 30 | 26 | 0x1cb0c88 | 2 | |
| 13 | 1232 | 20 | 9 | 0x9e7f71 | 4 | |
| 14 | 1248 | 1 | 1 | 0 | 1 | 兜底/占位 |

`be_get_structure_config`（func41）的 br_table 把 structType 映射到记录指针，
部分类型按 `mcVersion > 17`（即 1.18 分界）选择不同记录——需逐类型从 dcmp 抄录。

## 6. 已验证的 golden 样例（dump 脚本输出抽查）

- `1.20.0 / seed=12345`：spawn = **(226, 229)**；前 3 座要塞 (640,-320)、(96,832)、(-960,-192)；
  village region 列表 49 个，首个 (-1448,-1480)；village 配置 `[34, 26, 10387312, 4]`。
- golden 文件 `tests/fixtures/bedrock_golden.json` 的 `mt_vectors` 段数值疑似堆指针
  （`be_mt_n_get` 返回语义未澄清），**移植时先用 Node 探针搞清该函数再修正 dump 脚本**。

## 7. 算法要点（移植前必读）

- **RNG 是 MT19937**（`be_mt_n_get` 内部 `1812433253` 递推为铁证），与 Java 版 LCG
  完全不同；Bedrock 结构散布是「region 网格 + MT 随机偏移」路线，参数来自上面的配置表。
- spawn / strongholds 路径内部带一套 **Bedrock 分层群系评估**（dcmp 数据段有
  `mapRiverMix()/mapOceanMix()/mapHills() requires two parents! Use setupMultiLayer()`
  字符串，层名与 cubiomes/MC 旧版分层同族但参数不同），spawn 需要用它找合法群系。
- `be_get_filtered_structures_in_regions`（导出 p）做群系过滤，网站未使用；
  本库应实现非过滤版为主，过滤版可作为可选项（依赖内部群系评估）。
- dcmp 中 table T_a（29 项函数指针）疑似按结构类型分派的生成函数表。

## 8. 移植计划（下次执行的步骤）

1. 通读 `reference/site/bedrock.dcmp`（5595 行），重点 func10（MT）、func41（配置分派）、
   func28（region 散布）、func23/24（要塞/出生点）及其调用的层函数。
2. 用 Node 探针澄清 `be_mt_n_get` 语义，修正 `dump_bedrock_golden.mjs` 的 mt_vectors 段。
3. 新建 `src/bedrock/`：`BedrockVersion` 枚举、`BeStructureType` 枚举、MT19937、
   配置表（静态数组）、`structures_in_regions` / `spawn` / `strongholds`。
4. 生成 `tests/bedrock_golden_data.rs`（静态数组，参考 `tests/web_golden_data.rs` 风格），
   写 `tests/bedrock_consistency.rs` 对 91 用例精确比对。
5. `cargo test` 全绿 + `cargo clippy --all-targets -- -D warnings` 无警告。
6. 更新 README / docs/INTEGRATION.md（Bedrock 章节）。
7. git 提交并推送（见下节）。

建议顺序：MT19937 → 配置表 → 单结构 region 定位 → 全结构 → spawn → strongholds，
每步先用 golden 小范围验证再扩展。

## 9. 发布

```sh
git remote add origin git@github.com:TinggalLeaf/minecraft_seed_core.git
git branch -M main
git push -u origin main
```
