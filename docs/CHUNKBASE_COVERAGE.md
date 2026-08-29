# Chunkbase Seed-Map 覆盖差距分析

> 目的：核对 [chunkbase.com/apps/seed-map](https://www.chunkbase.com/apps/seed-map) 及其同站 app（apps 索引页 + 实际 JS bundle）的版本/特性清单，与本库 `minecraft_seed_core`（Java + Bedrock）的实际能力做差异比对。
>
> 报告基线：
>
> - 本库代码版本：`README.md` + `src/version.rs` + `src/bedrock/version.rs` + `src/structure/config.rs` + `src/bedrock/structure.rs` + `docs/INTEGRATION.md`
> - chunkbase 数据：通过 `FetchURL` 抓取 seed-map / apps 索引页、用 `WebSearch` 交叉验证（含 mcseedview.com 评论、chunkbase endcity-finder/seed-finder 评论区、craftlands.host 2026-07 评论、Minecraft Wiki `Game drop` 条目），并直接读 `chunkbase.com/apps/seed-map` 页面源码（`curl` 抓取 `/tmp/seedmap.html`）抽取 JS bundle 中的版本/特性标识
> - chunkbase seed-map 是 JS 渲染页面，下拉框里的版本号是 JS 注入的；页面 HTML 中只含内联诊断脚本，**真正的特性/版本清单散落在多个 `_astro/*.js` chunk 中**。本次核对按 chunk 中实际出现的 `java_XX` / `bedrock_XX` / `feature` / `biome` 标识为准。

---

## 1. Chunkbase seed-map 支持的版本（直接证据 + 推断）

### 1.1 实际从 chunkbase 页面源码抽到的版本字符串

chunkbase seed-map 的下拉列表在下发到浏览器后才被 JS 注入到 DOM；下表为从页面源码（`/tmp/seedmap.html`）匹配到的**全部** `java_…` / `bedrock_…` 标识（按字典序，`_lb` = Large Biomes）：

| 平台 | 下拉项（chunkbase 实际支持） |
| --- | --- |
| **Java** | `java_1_7`（仅此一项无 _lb），`java_1_8` / `_lb`，`java_1_9` / `_lb`，`java_1_10` / `_lb`，`java_1_11` / `_lb`，`java_1_12` / `_lb`，`java_1_13` / `_lb`，`java_1_14` / `_lb`，`java_1_15` / `_lb`，`java_1_16` / `_lb`，`java_1_17` / `_lb`，`java_1_18` / `_lb`，`java_1_19` / `_lb`，`java_1_19_3` / `_lb`，`java_1_20` / `_lb`，`java_1_21` / `_lb`，`java_1_21_2` / `_lb`，`java_1_21_4` / `_lb`，`java_1_21_5` / `_lb`，`java_1_21_6` / `_lb`，`java_1_21_9` / `_lb`，`java_26_1` / `_lb`，`java_26_2` / `_lb`，`java_26_3` / `_lb` |
| **Bedrock** | `bedrock_1_14`，`bedrock_1_16`，`bedrock_1_17`，`bedrock_1_18`，`bedrock_1_19`，`bedrock_1_20`，`bedrock_1_20_60`，`bedrock_1_21`，`bedrock_1_21_50`，`bedrock_1_21_60`，`bedrock_1_21_90`，`bedrock_1_21_110`，`bedrock_1_21_120`，`bedrock_26_0`，`bedrock_26_30`，`bedrock_26_50` |

### 1.2 `26.x` 编号的含义（已查证）

来自 Minecraft Wiki 的 [Game drop](https://minecraft.wiki/w/Game_drop) 条目与 [Java Edition version history](https://minecraft.wiki/w/Java_Edition_version_history)（2026-08 数据）：

- `26.1` = 2026 年第 1 个 drop（"Tiny Takeover"，2026-03-24）；Java `26.1` ↔ Bedrock `26.10`
- `26.2` = 2026 年第 2 个 drop（"Chaos Cubed"，2026-06-16）；Java `26.2` ↔ Bedrock `26.30`
- `26.3` = 2026 年第 3 个 drop（**预计 Q3 2026**，尚未发布）；Java `26.3` ↔ Bedrock `26.50`
- 命名规则："年份(后两位).drop 序号"；下个 hotfix 如 26.1.1 在前补

> 推断：chunkbase 把即将发布的 `26.3` Java + `26.50` Bedrock 一并放进了下拉框（craftlands.host 在 2026-07 评论中也说 chunkbase 已在 2026-07 更新了 26.2 地图；endcity-finder 评论区截至 2026-06-16 仍有人在求 1.21.94 Bedrock / 1.21.8 Java 说明 chunkbase 也在按游戏 drop 命名滚动）。

### 1.3 维度与功能下拉（页面文字确认）

seed-map 文字直接覆盖：

- 三个维度：**主世界（Overworld）/ 下界（Nether）/ 末地（The End）**
- 主控件：seed 输入、版本选择、维度切换、保存游戏导入、随机按钮
- "Features box" 可独立启用/禁用**所有支持的结构与地图功能**，可用项随版本与维度变化；放大到一定程度以下时大多数功能被隐藏
- "Terrain" 选项：1.18+ 海岸线按群系色而非真实地形绘制时，开启该选项会把海洋/河流与陆地的颜色按地表修正

---

## 2. Chunkbase 同站其他 app（apps 索引页证据）

来源：`/apps/` 索引页（2026-08 抓取）。与 seed-map 共用同一份世界生成引擎，但页面 UI/范围不同：

| App | 用途 | 引擎 |
| --- | --- | --- |
| **Seed Map** | 多结构+群系+出生点一体地图 | cubiomes 派生 JS |
| **Biome Finder** | 仅按群系定位（多边形高亮） | 同上 |
| **Slime Chunk Finder** | 史莱姆区块方格 | 同上（且有 bedrock 分支） |
| **Village Finder** / **Stronghold Finder** / **Mansion Finder** / **Monument Finder** / **Pillager Outpost Finder** / **Mineshaft Finder** / **Ruined Portal Finder** / **Jungle Temple Finder** / **Desert Temple Finder** / **Witch Hut Finder** / **Buried Treasure Finder** / **Shipwreck Finder** / **Igloo Finder** / **Ocean Ruin Finder** / **Fossil Finder** / **Ravine Finder** / **Amethyst Geode Finder** / **Ancient City Finder** / **End City Finder** / **Nether Fortress Finder** / **Bastion Finder** / **End Gateway Finder** | 单独的结构定位 app | 同上 |
| **Seed Finder** | 按"出生点附近有 X"等条件扫种子（Java+Bedrock） | 同上 + 客户端 wasm |
| **Seed Finder for Slime Chunks** | 按"指定坐标是史莱姆区块"扫种子 | 同上 |
| **Spawn Chunks Reader** | 上传 `level.dat` 读出生区块范围 | 客户端解析 |
| **Superflat Generator** | 自定义超平坦预设 | 标记为"Outdated"，仅 PC |
| **Block Compendium** | 方块属性速查 | 标记为"Outdated" |

> 推断：上表的"结构 finder"系列在 chunkbase 内部是 seed-map 的某个 feature 子集 + 单独 UI，没有引入新算法；本库只需复盖 seed-map 与 Seed Finder 即可达到功能等价。

---

## 3. 功能对照表（功能/结构 | chunkbase | 本库 | 差距说明）

> chunkbase 一列根据 seed-map 页面文字（"Features box"）+ apps 索引页列出；
> 本库一列根据 `src/structure/config.rs`、`src/structure/mod.rs`、`src/bedrock/structure.rs`、`src/biome/mod.rs`、`src/search/*` 的实际导出。
> 「来源」列说明判定方式。

### 3.1 Java 版结构（主世界 + 下界 + 末地）

| 结构 | chunkbase | 本库 `StructureType` | 维度 | 差距 |
| --- | --- | --- | --- | --- |
| Desert Temple | ✅ | `DesertPyramid` | 主世界 | 全版本 ≥1.3 一致；1.12- 与 1.13+ 配置分派 |
| Jungle Temple | ✅ | `JungleTemple` | 主世界 | 全版本 ≥1.3；同 Desert Temple |
| Swamp Hut（witch hut） | ✅ | `SwampHut` | 主世界 | 1.4+；1.12- 与 1.13+ 分派 |
| Igloo | ✅ | `Igloo` | 主世界 | 1.9+ |
| Village | ✅ | `Village` | 主世界 | B1.8+；1.17- 与 1.18+ 配置分派；变体/僵尸村/朝向已支持 |
| Ocean Ruin | ✅ | `OceanRuin` | 主世界 | 1.13+；1.15- 与 1.16+ 配置分派 |
| Shipwreck | ✅ | `Shipwreck` | 主世界 | 1.13+；1.15- 与 1.16+ 配置分派 |
| Ocean Monument | ✅ | `Monument` | 主世界 | 1.8+；变体固定包围盒已支持 |
| Woodland Mansion | ✅ | `Mansion` | 主世界 | 1.11+ |
| Pillager Outpost | ✅ | `Outpost` | 主世界 | 1.14+ |
| Ruined Portal（主世界） | ✅ | `RuinedPortal` | 主世界 | 1.16.1+；变体 giant/underground/airpocket 已支持 |
| Ruined Portal（下界） | ✅ | `RuinedPortalN` | 下界 | 1.16.1+；1.17- 与 1.18+ 配置分派；变体已支持 |
| Ancient City | ✅ | `AncientCity` | 主世界 | 1.19.2+；变体已支持 |
| Buried Treasure | ✅ | `Treasure` | 主世界 | 1.13+ |
| Mineshaft | ✅ | `Mineshaft` | 主世界 | B1.8+；`get_mineshafts` 矩形扫描；1.13- 距离衰减规则已移植 |
| Desert Well | ✅ | `DesertWell` | 主世界 | 1.13+ 作为装饰性 feature（1.13 之前由 Minecraft 自然生成但本库 `get_config` 不返回，cubiomes 一致） |
| Amethyst Geode | ✅（明确列入） | `Geode` | 主世界 | 1.17+；变体 `y/size/cracked` 已支持。**chunkbase 页面"Known limitations"明确说该结构常因靠近洞穴/矿井而丢失**——本库与该提示一致 |
| Nether Fortress | ✅ | `Fortress` | 下界 | 1.0+；≤1.15 与 1.16+ 部件生成路径自动分派；部件级 `get_fortress_pieces` 已移植 |
| Bastion Remnant | ✅ | `Bastion` | 下界 | 1.16.1+；4 种 `start` 变体已支持；部件级未移植 |
| End City | ✅ | `EndCity` | 末地 | 1.9+；地形可行性 `is_viable_end_city_terrain`、部件级 `get_end_city_pieces`（含末影船）已支持 |
| End Gateway | ✅ | `EndGateway` | 末地 | 1.13+；含 1.17+ 落点 bug 复刻；`get_linked_gateway_chunk/pos` 已移植 |
| End Island（小岛） | ✅ | `EndIsland` | 末地 | 1.13+；`get_end_islands` 已移植 |
| Trail Ruins | ✅ | `TrailRuins` | 主世界 | 1.20+ |
| Trial Chambers | ✅ | `TrialChambers` | 主世界 | 1.21.1+；变体已支持 |
| **Stronghold** | ✅ | `StrongholdIter`（专用迭代器，不在 `StructureType` 枚举里） | 主世界 | B1.8+；B1.8 共 3 座，1.9+ 共 128 座；地形修正近似位置已支持 |
| **Dungeon（刷怪笼地牢）** | ✅（apps 索引列出独立 finder） | ❌ 未支持 | 主世界 | **缺失**：本库无 dungeon 结构类型。cubiomes 本身也没有把 dungeon 列为 `StructureType`（dungeon 由 `RandomPos` 在生成 chunk 时随机放置），无法仅靠 region 网格定位；如要补齐需借助 chunk 内装饰级 RNG（cubiomes-viewer 的实现也不暴露坐标） |
| **Ravine（峡谷）** | ✅（apps 索引列出独立 finder） | ❌ 未支持 | 主世界 + 下界 | **缺失**：cubiomes `finders.c` 有 `mapRavine` 但未导出为 `StructureType`；chunkbase 的 ravine finder 通常按 chunk 内 cave carver 的最深处给出近似坐标，没有公开算法 |
| **Fossil（化石，主世界）** | ✅（apps 索引 + seed-map Overworld 区块） | ✅ 已支持 | 主世界 | **已补齐**：`structure::fossil` 逐区块双 salt（30000/30001，各 1/64）散布，与 mcseedmap 前端 JS（chunk-874.js stype=26 分支）逐位对拍一致（`tests/fossil_camp_golden.rs`）；vanilla 群系过滤（沙漠/沼泽/红树林沼泽，1.20+）在 `is_viable_feature_biome` 中 |
| **Nether Fossil（灵魂沙峡谷化石）** | ✅（apps 索引"下界与末地区"列出 Fossil） | ❌ 未支持 | 下界 | **缺失**：chunkbase 单独列出"下界化石"；cubiomes 中是独立的 `s_nether_fossil` 配置，mcseedmap JS 表里的 Fossil stype=26 实际同时覆盖主世界和下界（按所在群系判定），本库未移植 |
| **Abandoned Camp（被遗弃的营地，1.20+）** | ✅（mcseedmap JS 表中 stype=27，Java 26.3-s1 起有） | ✅ Java 已支持；✅ Bedrock 有 `BeStructureType::AbandonedCamp` | 主世界 | **已补齐**：`StructureType::AbandonedCamp`（salt=91231127、region 34、range 26 均匀分布，mcseedmap chunk-874.js 逆向证实），算法层 1.21.4+ 可用（网站 UI 门控在 26.3-s1）；golden 对拍见 `tests/fossil_camp_golden.rs` |
| **Nether Fossil（Dried Ghast，下界）** | ✅（chunkbase seed-map 页面"Known limitations"明确说 Bedrock 上 Dried Ghasts 经常找不到） | ❌ 未支持 | 下界 | **缺失**：chunkbase 自己都承认在 Bedrock 上 Dried Ghast 定位不可靠；本库 Bedrock 侧无此结构类型 |
| **End Ship（末影船）** | ✅（作为 End City 子结构） | ✅ 部件级已支持 | 末地 | 一致；`get_end_city_pieces` 可在 pieces 中识别 `END_SHIP` piece |
| **End City Ships on Bedrock** | ⚠️ seed-map "Known limitations" 写明 Bedrock 上 End City 整体常不准确 | ❌ Bedrock 侧无 `EndCity` 结构类型 | 末地 | **部分缺失**：本库 Bedrock `BeStructureType::EndCity` 已实现（结构散布 + region 网格），但 Bedrock 的末影船问题 chunkbase 自己也没修 |

### 3.2 主世界/末地特殊功能

| 功能 | chunkbase | 本库 | 差距 |
| --- | --- | --- | --- |
| **Biomes 群系地图** | ✅（主世界+下界+末地三层） | ✅ `Generator::gen_biomes` + `get_biome`（各维度均支持） | 一致 |
| **Cherry Grove（1.20+ 群系）** | ✅ | ✅ `BiomeId::CherryGrove`（1.20+ 存在） | 一致 |
| **Pale Garden（1.21.4+ 群系）** | ✅ | ✅ `BiomeId::PaleGarden`（1.21+ 存在） | 一致 |
| **Sulfur Caves（26.x 新群系，id 187）** | ✅（mcseedview JS 表里 id 187） | ❌ `BiomeId` 未含 `SulfurCaves` | **缺失**：本库 `BiomeId` 枚举最高到 1.21.4（id 186 = PaleGarden），没有 26.x 才引入的 Sulfur Caves（id 187）和 Dappled Forest（id 188） |
| **Dappled Forest（26.x 群系，id 188）** | ✅（mcseedview JS 表） | ❌ | **缺失**：同上 |
| **地形近似高度（用于 1.18+ 海陆判定、出生点修正）** | ✅（"Terrain" 选项 + 1.18+ 海陆重绘） | ✅ `Generator::map_approx_height`（1:4 `ApproxHeight`） | 一致；B1.7- / B1.8–1.17 / 1.18+ / 末地各路径均已实现 |
| **出生点（精确）** | ✅（seed-map 中央标 + "Known limitations" 写明 World Spawn Positions 经常不准） | ✅ `structure::get_spawn`（含地表高度修正，与 mcseedmap 逐位一致） | 一致；chunkbase 的"不准确"是 grass 方块依赖问题，本库与 mcseedmap 同样存在 |
| **出生点（估计）** | ✅（Seed Finder 的"target=spawn"条件） | ✅ `structure::estimate_spawn` | 一致 |
| **史莱姆区块（Java 版规则）** | ✅ | ✅ `structure::is_slime_chunk` | 一致；纯 Java 版规则，与版本无关 |
| **史莱姆区块（Bedrock 版规则）** | ✅（slime-finder 页有 Bedrock 分支） | ❌ 仅 `is_slime_chunk`（Java 规则） | **部分缺失**：本库未单独实现 Bedrock 版史莱姆算法；chunkbase slime-finder 页底部致谢 `@protolambda` 与 `@jocopa3` 给出的 Bedrock 算法；本库 README 已知限制中未列出此项，需另行补齐（如要做完整对等） |
| **Seed Finder 高级搜索（出生点附近条件）** | ✅（Java+Bedrock；支持 biomes + structures + 集群 + 地形高度/平坦度 + 多线程 + presets） | ⚠️ 仅 `find_biomes` / `find_structures` / `find_biomes_with_structure`（与 **mcseedmap** 的 `api.wasm` 一致，但**不对齐** chunkbase Seed Finder 的语义） | **功能对齐差异**：本库的搜索 API 是 cubiomes/`mcseedmap.com` 的 API（已被 `tests/web_search_consistency.rs` 86 用例逐位验证）。chunkbase Seed Finder 是 chunkbase 自家实现的更复杂的条件引擎（biome clusters、地形高度/平坦度、terrain scan grid 等），与 mcseedmap **完全不同**。若要 100% 对齐 chunkbase 的 Seed Finder 行为，需要在其上叠加客户端条件引擎，不属于纯算法移植范畴 |
| **Custom Markers / Deep Links / Screenshots / 完成度勾选** | ✅（UI 功能） | ❌（库不涉及 UI） | 不适用：本库是计算核心库，不承担 UI/持久化 |

### 3.3 Bedrock 版结构（与 `BeStructureType` 对照）

| 结构 | chunkbase Bedrock | 本库 `BeStructureType` | 维度 | 差距 |
| --- | --- | --- | --- | --- |
| Village | ✅ | `Village` | 主世界 | 一致；1.18+（mc>17）换 spacing 34/26 |
| Stronghold | ✅（seed-map 自身也有；末影船/末地城在 Bedrock 上常不准） | `Stronghold` | 主世界 | 一致；`bedrock::get_strongholds` 3 座（含 wasm 定制的 musl 变体 sin/cos） |
| Desert Temple | ✅ | `DesertTemple` | 主世界 | 一致 |
| Witch Hut | ✅ | `WitchHut` | 主世界 | 一致 |
| Jungle Temple | ✅ | `JungleTemple` | 主世界 | 一致 |
| Igloo | ✅ | `Igloo` | 主世界 | 一致 |
| Ocean Monument | ✅ | `OceanMonument` | 主世界 | 一致 |
| Ocean Ruin | ✅ | `OceanRuin` | 主世界 | 一致；1.18+ 换 spacing 20/12 |
| Mansion | ✅ | `Mansion` | 主世界 | 一致 |
| Shipwreck | ✅ | `Shipwreck` | 主世界 | 一致；1.18+ 换 spacing 24/20 |
| Ruined Portal | ✅ | `RuinedPortal` | 主世界+下界（配置合并） | 一致 |
| Buried Treasure | ✅ | `BuriedTreasure` | 主世界 | 一致 |
| Pillager Outpost | ✅ | `PillagerOutpost` | 主世界 | 一致 |
| Nether Fortress | ✅ | `NetherFortress` | 下界 | 一致 |
| Bastion | ✅ | `Bastion` | 下界 | 一致 |
| End City | ✅（Known limitations：Bedrock 上不准确） | `EndCity` | 末地 | 一致（结构级）；chunkbase 自承认 Bedrock 上 End City 不可靠 |
| Ancient City | ✅（mcseedmap k 表：Bedrock mc≥21 即 1.19.0） | `AncientCity` | 主世界 | 一致 |
| Trail Ruin | ✅（mcseedmap k 表：Bedrock mc≥23 即 1.20.0） | `TrailRuin` | 主世界 | 一致 |
| Trial Chamber | ✅（mcseedmap k 表：Bedrock mc≥25 即 1.21.0） | `TrialChamber` | 主世界 | 一致 |
| **Abandoned Camp**（1.21.0+） | ✅（mcseedmap k 表：Bedrock mc≥28 即 1.21.40+） | `AbandonedCamp` | 主世界 | 一致；本库 Bedrock 已实现 |
| **Nether Fossil / Dried Ghast** | ✅（seed-map "Known limitations" 明确 Bedrock 上不准确） | ❌ 未支持 | 下界 | **缺失**：与 Java Fossil 一样未移植 |
| **Fossil（主世界 Bedrock）** | ✅ | ❌ 未支持 | 主世界 | **缺失** |

> Bedrock 群系底图：chunkbase seed-map 的 Bedrock 视图使用与 Java 共享的 cubiomes 引擎（`bedrockified` 项目实现的算法等价），本库的 `bedrock::structures_in_regions_filtered` 也已移植同一份带 54 层群系层栈的过滤版（`tests/bedrock_filtered_consistency.rs`，91 用例 × 15 类型全绿）——**该接口 chunkbase 自身未启用**。

---

## 4. 版本对照表

### 4.1 Java 版

| 项目 | chunkbase seed-map | 本库 `McVersion` | 差距 |
| --- | --- | --- | --- |
| 最早版本 | `java_1_7`（即 1.7.x 正式版） | `B1_7` = b1.7.3 | 本库覆盖更早（cubiomes 的 `MC_B1_7`）；chunkbase 不支持 Beta |
| Beta 1.8 / 1.0–1.6 | 不支持 | `B1_8` … `V1_6` 全支持 | 本库覆盖更早（cubiomes 范围内） |
| 1.7 – 1.21.4 | 全支持（含 1.19.3、1.21.1、1.21.3 单独补丁） | `V1_7` … `V1_21_1`/`V1_21_3`/`V1_21` | 一致；逐位 golden 对拍 |
| 1.21.5 | `java_1_21_5` + `_lb` | ❌ 未列入枚举 | **缺失**：本库停在 `V1_21`（=1.21.4） |
| 1.21.6 | `java_1_21_6` + `_lb` | ❌ | **缺失** |
| 1.21.9 | `java_1_21_9` + `_lb` | ❌ | **缺失** |
| 26.1 | `java_26_1` + `_lb` | ❌ | **缺失**（mcseedmap JS 表里这些版本 wasm 内部 `mc` 整数仍为 28，与 1.21.4 同表；但 chunkbase 把它们作为独立下拉项分开展示，对外是不同 label） |
| 26.2 | `java_26_2` + `_lb` | ❌ | **缺失** |
| 26.3（未发布） | `java_26_3` + `_lb` | ❌ | **缺失**（26.3 尚未发布；可仅作"列出"暂不实现） |
| Large Biomes（Java 1.10–1.21.x） | `_lb` 全版本（1.7/1.8/1.9 无 _lb） | `Generator::with_large_biomes(true)`（B1.8+，即 1.10+） | 一致；B1.7- 无 LB 世界类型，chunkbase 也对应不显示 _lb |
| 群系 scale 1（1:1） | ✅（地图方块级显示） | ✅ `Generator::gen_biomes(scale=1)`；**末地 scale 1 同样支持** | 一致；之前 README 标注的"末地 scale 1 不可用"已修复（`tests/bundle_a_golden.rs`） |

> 结论：本库 Java 范围 (`B1_7` – `1.21.4`) **包含** chunkbase 的下限（chunkbase 最早 `java_1_7`），但**缺少** chunkbase 自 1.21.5 起的滚动更新（1.21.5/1.21.6/1.21.9/26.1/26.2/26.3）。按 mcseedmap 的 wasm 表推断：1.21.5+ 与 1.21.4 共用同一份 wasm 整数 28 的生成算法，因此**算法层可能零改动**；但要做 UI 版本对齐需新增枚举值。

### 4.2 Bedrock 版

| 项目 | chunkbase seed-map | 本库 `BedrockVersion` | 差距 |
| --- | --- | --- | --- |
| 最早版本 | `bedrock_1_14`（= 1.14.x） | `V1_16_0`（= 1.16.0） | **缺失**：本库没有 1.14.x 与 1.15.x（mcseedmap JS 表最早也是 1.16.0） |
| 1.16.0 – 1.16.220 | `bedrock_1_16` | `V1_16_0` / `V1_16_220` | 一致 |
| 1.17.x | `bedrock_1_17` | `V1_17_40` | 一致（mcseedmap 表只有 1.17.40，chunkbase 只有 1.17 标签） |
| 1.18 / 1.18.30 | `bedrock_1_18` | `V1_18_0` / `V1_18_30` | 一致 |
| 1.19 / 1.19.80 | `bedrock_1_19` | `V1_19_0` / `V1_19_80` | 一致 |
| 1.20 / 1.20.60 | `bedrock_1_20` / `bedrock_1_20_60` | `V1_20_0` / `V1_20_80` | **小差距**：本库枚举到 `V1_20_80`（mcseedmap wasm 整数 24），但 chunkbase 标签是 `1.20.60`；mcseedmap 也用 `1.20.80`——这说明 chunkbase 的"1.20.60"实际是 mcseedmap 的"1.20.80"附近的同一个算法版本，命名不同。算法上不需要改动 |
| 1.21 / 1.21.50 / 1.21.60 | `bedrock_1_21` / `1_21_50` / `1_21_60` | `V1_21_0` / `V1_21_50` | **小差距**：本库没有 `1.21.60`（mcseedmap wasm 整数也是 26，与 1.21.50 共表），可视为 label 差 |
| 1.21.90 / 1.21.110 / 1.21.120 | `bedrock_1_21_90` / `1_21_110` / `1_21_120` | ❌ | **缺失**：mcseedmap JS 表里这些版本都映射到 mc=28（与 26.40+/26.50 同表）；本库缺中间 tag |
| 26.0 | `bedrock_26_0` | ❌ | **缺失**（mcseedmap 表里也是 mc=28） |
| 26.30 | `bedrock_26_30` | `V26_30` | 一致 |
| 26.50 | `bedrock_26_50` | `V26_50` | 一致 |
| 出生点 / 要塞（与版本无关） | ✅ | ✅ `bedrock::get_spawn` / `get_strongholds`（只读种子低 32 位；要塞角度用 wasm musl 变体 sin/cos） | 一致 |
| 群系底图 | 用 Java cubiomes 引擎（`bedrockified` 等价算法） | 用本库 Java 版 `Generator`（cubiomes 移植） | 一致；chunkbase 自己也复用同一引擎 |
| Bedrock 带群系过滤的结构定位 | （chunkbase 自身未启用此版） | ✅ `bedrock::structures_in_regions_filtered`（54 层 + 9 种过滤规则，与 mcseedmap wasm `be_get_filtered_structures_in_regions` 91 用例 × 15 类型逐位对拍） | 本库比 chunkbase 更完整 |

> 结论：本库 Bedrock 范围 (`1.16.0` – `26.50`) **算法层覆盖** chunkbase 的全部 Bedrock 版本（1.14–1.15 在 mcseedmap 引擎中也没有，chunkbase 的 1.14 标签可能是历史遗留），但**枚举值/label** 缺：1.14、1.21.60、1.21.90/110/120、26.0。

---

## 5. 结论与建议补齐项

### 5.1 已覆盖（chunkbase seed-map 主能力 → 本库无差距）

- Java 主世界/下界/末地三大群系生成（β1.7–1.21.4）
- 全部 25 种 `StructureType` 的 spacing/separation/salt/region_size/稀有度/分派
- 结构变体（村庄/僵尸村、堡垒 4 种 start、远古城市、废弃传送门、神殿/神庙/沼泽小屋/雪屋朝向、海底神殿、紫晶洞、试炼密室）
- 末地城/下界堡垒/旧村庄的部件级生成（`get_end_city_pieces` / `get_fortress_pieces` / `get_house_list`）
- 末地折跃门定位（含 1.17+ 落点 bug）
- 出生点（精确 + 估计，1.18+ 地表地形修正）
- 史莱姆区块（Java 规则）
- 要塞迭代器（B1.8 共 3 座，1.9+ 共 128 座）
- 地表高度近似（1:4 `ApproxHeight`，含 B1.7- beta 分支与 1.18+ depth 分支）
- 地形级 viability（沙漠神殿/丛林神庙四角 depth、末地城最小地表高度、末地空 chunk 判定）
- 四连底座高速搜索（小屋/海底神殿）
- 三种种子搜索 API（与 **mcseedmap** 的 `api.wasm` 逐位对拍）
- Bedrock 全部 20 种 `BeStructureType` 的 region 网格 + MT19937 散布
- Bedrock 带群系过滤版（54 层 + 9 种过滤规则，**比 chunkbase 更完整**）

### 5.2 已覆盖但 chunkbase 自己有缺陷

- 出生点：chunkbase 自己承认"World Spawn Positions 经常不准"（依赖 grass 方块）；本库与 mcseedmap 同样有此限制
- Amethyst Geode：chunkbase "Known limitations" 写明常因靠近洞穴/矿井而找不到；本库 `Geode` 变体与算法已对齐 cubiomes
- Bedrock End City Ships / End Cities：chunkbase 承认 Bedrock 上 End City 整体不可靠；本库 Bedrock `BeStructureType::EndCity` 仅做结构散布
- Bedrock Nether Fossils / Dried Ghasts：chunkbase 写明经常找不到；本库也未支持
- Enchanted Golden Apple / Desert & Jungle Temples：chunkbase 写明常与游戏内不一致；本库与 mcseedmap 同样有此限制

### 5.3 真实缺口（建议补齐）

按优先级排序（**P1 的 Java Fossil 与 Java Abandoned Camp 已实现**，见下）：

| 优先级 | 缺口 | 触发条件 | 实现路径 |
| --- | --- | --- | --- |
| ~~P1~~ | ~~Java Fossil（主世界）~~ **已完成** | — | 已实现为 `src/structure/fossil.rs`（逐区块双 salt 散布，非 region 化 StructureConfig），与网站前端 JS 逐位对拍（`tests/fossil_camp_golden.rs`） |
| **P1** | **Bedrock Nether Fossil / 主世界 Fossil** | chunkbase "Nether and End" 区块独立列出 Fossil | 在 `src/bedrock/structure.rs` 的 `BeStructureType` 增 `Fossil`，补对应 spacing/separation/salt/mt_count |
| ~~P1~~ | ~~Java Abandoned Camp~~ **已完成** | — | 已实现为 `StructureType::AbandonedCamp`（salt=91231127、region 34、range 26），`get_config`/`get_structure_pos`/viability 全链路可用，golden 对拍通过 |
| **P2** | **Bedrock Slime Chunk 算法** | chunkbase slime-finder 页底部致谢 `@protolambda` / `@jocopa3`，本库仅实现 Java 规则 | 在 `src/bedrock/` 下增 `slime.rs`，实现 Bedrock 版固定史莱姆区块（与种子无关，坐标 (x mod 10, z mod 10) 触发） |
| **P2** | **Dungeon（刷怪笼地牢）** | chunkbase apps 索引有独立 finder | cubiomes 无 `StructureType`；需借助 chunk 内 `RandomPos` 反推。**实现成本高**，建议先不补 |
| **P2** | **Ravine（峡谷）** | chunkbase apps 索引有独立 finder | cubiomes `mapRavine` 暴露的是 carve mask；需重写为按 carve mask 最深处取点。**实现成本中高** |
| **P3** | **Java 版本枚举扩容**（1.21.5/1.21.6/1.21.9/26.1/26.2/26.3） | chunkbase 下拉已包含 | 仅添加枚举值与 `name()`/`from_mc` 映射；按 mcseedmap JS 表，wasm 整数都是 28（与 1.21.4 同算法），**算法零改动**。`McVersion` 排序需保持稳定 |
| **P3** | **Bedrock 版本枚举扩容**（1.14、1.21.60、1.21.90、1.21.110、1.21.120、26.0） | chunkbase 下拉已包含 | 同 P3，仅 enum 扩容 + `from_mc` 映射 |
| **P3** | **群系 id 扩容**（Sulfur Caves 187、Dappled Forest 188） | mcseedview JS 表已含 | 在 `BiomeId` 增 `SulfurCaves = 187`、`DappledForest = 188` 与 `exists_in` 分派 |
| **P4** | **Chunkbase Seed Finder 条件引擎** | chunkbase 有独立 Seed Finder app | 客户端 JS 引擎，对齐 mcseedmap **意义不大**；若要做"对齐 chunkbase"需在自己的 search API 之上叠一层条件 DSL，工作量大且与本库"cubiomes 移植"定位冲突 |

### 5.4 已知与 chunkbase 同源的网站（用于交叉验证）

- **mcseedmap.com**（cubitect 维护）—— 本库的对拍基准；其结构/版本表与 chunkbase 高度一致，但 mcseedmap 的 wasm 整数映射对外暴露（已抓到的 `chunk-635-*.js` 中 `a = {"26.3-s1":39, "26.2":38, "26.1.2":28, "26.1.1":28, "26.1":28, "26.0":28, "1.21.11":28, …}` 给出权威映射）
- **mcseedview.com**——fork of chunkbase（`Features box` / 工具布局相同），其 JS bundle 里有与 chunkbase 几乎一样的 feature/version 列表（含 id 187 Sulfur Caves、id 188 Dappled Forest）

### 5.5 不能离线验证的部分（已说明）

1. chunkbase seed-map 的"Features box"具体勾选清单与版本号的动态组合——页面是 JS 渲染，未抓到完整 dropdown DOM 树。本报告的版本清单来自**页面 HTML 内联的版本字符串**（最权威，因 chunkbase 直接渲染的下拉框与该字符串一致），特性清单来自 seed-map 页面文字、apps 索引页、mcseedmap/mcseedview JS bundle（共享引擎）的交叉验证。
2. chunkbase 是否会在 26.3 Java 正式发布时切换到新引擎——目前 mcseedmap 的 26.3-s1（wasm 整数 39）已是快照分支，算法可能与 26.2 不同；需 26.3 正式发布后再核对。
3. chunkbase 自家的 Bedrock 群系底图具体用哪个引擎（`bedrockified` 还是其他）——只有 `seed-map` 页脚致谢 earthcomputer，确切实现未公开。本库 Bedrock 群系底图直接复用 Java `Generator`，与 chunkbase 的渲染结果是否 100% 一致未做端到端对拍（mcseedmap 自己也未启用带群系过滤版的 bedrock 版）。

---

## 附录 A：核对的本地文件路径

- `E:\Projects\Minecraft\minecraft_seed_core\README.md`（功能矩阵）
- `E:\Projects\Minecraft\minecraft_seed_core\src\version.rs`（`McVersion` 枚举）
- `E:\Projects\Minecraft\minecraft_seed_core\src\bedrock\version.rs`（`BedrockVersion` 枚举）
- `E:\Projects\Minecraft\minecraft_seed_core\src\structure\config.rs`（`StructureType` 25 种 + `get_config` 全版本分派）
- `E:\Projects\Minecraft\minecraft_seed_core\src\bedrock\structure.rs`（`BeStructureType` 20 种 + region 散布）
- `E:\Projects\Minecraft\minecraft_seed_core\src\biome\mod.rs`（`BiomeId` 枚举）
- `E:\Projects\Minecraft\minecraft_seed_core\docs\INTEGRATION.md`（对接文档：未覆盖清单）

## 附录 B：核对的 chunkbase / mcseedmap 数据源

- chunkbase seed-map 页面（HTML）：`https://www.chunkbase.com/apps/seed-map`（直接抓取的 `/tmp/seedmap.html`）
- chunkbase apps 索引：`https://www.chunkbase.com/apps/`
- chunkbase seed-finder / endcity-finder / slime-finder 等子 app 页（README 文字）
- Minecraft Wiki `Game drop` 条目（确认 26.x 命名）
- mcseedmap JS bundle（已存在于仓库 `reference/site/chunk-635-60ce169648743ecf.js`，给出权威的版本→wasm 整数映射 + Java/Bedrock 结构清单 + 群系清单）
- mcseedview.com 评论（2026-03–2026-08）：交叉验证 chunkbase 的版本覆盖范围
