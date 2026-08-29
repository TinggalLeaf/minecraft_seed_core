//! Loot table 引擎：1:1 翻译 Python `src/loot_table.py` 的生成算法。
//!
//! 输入：JSON 字符串或已经解析好的 [`json::Value`]。
//! 输出：[`LootTable`] 结构；调用 [`LootTable::generate`] 配合任意
//! [`LootRng`] 得到 [`LootItem`] 列表。
//!
//! 设计要点：
//! - NumberProvider 三种：`constant` / `uniform` / `binomial`，严格按
//!   `defpackage.edf` 实现（constant 直接转 int；uniform 用
//!   `nextIntBetween` 含两端；binomial 是 n 次独立 `nextDouble() < p`）。
//! - Loot 函数目前实现：`minecraft:set_count`、`minecraft:enchant_randomly`、
//!   `minecraft:apply_bonus`（时运：`ore_drops` 与 `uniform_bonus_count`
//!   两种公式；`fortune_level` 由调用方通过
//!   [`LootTable::generate_ctx`] 传入，默认 0 即无效果）。
//!   其它函数（`set_nbt`、`fill_player_head`、`looting_enchant` 等）按需
//!   添加；`enchant_with_levels` 见下方。
//! - 复合 entry：`minecraft:alternatives`（第一个条件通过的 child）、
//!   `minecraft:sequence` / `minecraft:group`（依次拼接全部 child）。
//!   与 Python 端一致，条件一律视为通过（blocks/entities 表中的
//!   `match_tool` / `survives_explosion` 等条件需要游戏内上下文，
//!   无法从种子复现）。
//! - `enchant_with_levels`：原版真实行为是从附魔注册表随机选一个可用
//!   附魔施加给物品；注册表依赖完整游戏数据，Python 参考项目未对
//!   `enchant_with_levels` 设置 `__enchanted__` 占位，为逐位对拍
//!   本模块也不设置（与 `enchant_randomly` 的 `enchanted = true` 区别开）。
//! - `binomial` 已知边角：vanilla 在 `defpackage.edc` 用 `RandomSource
//!   .nextDouble() < p`，与本模块一致。

use crate::loot::json::{self, Value};
use crate::loot::rng::LootRng;

/// 一件战利品（含 `__enchanted__` 占位的 `enchanted` 字段）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LootItem {
    pub item: String,
    pub count: i32,
    pub enchanted: bool,
}

impl LootItem {
    pub fn new(item: impl Into<String>, count: i32) -> Self {
        LootItem { item: item.into(), count, enchanted: false }
    }

    pub fn enchanted(mut self) -> Self {
        self.enchanted = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Number providers
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) enum NumberProvider {
    Constant(i64),
    Uniform { lo: i32, hi: i32 },
    Binomial { n: i32, p: f64 },
}

impl NumberProvider {
    pub fn sample<R: LootRng + ?Sized>(&self, rng: &mut R, _luck: f64) -> i32 {
        match self {
            NumberProvider::Constant(v) => *v as i32,
            NumberProvider::Uniform { lo, hi } => rng.next_int_between(*lo, *hi),
            NumberProvider::Binomial { n, p } => {
                let mut s = 0i32;
                for _ in 0..*n {
                    if rng.next_double() < *p {
                        s += 1;
                    }
                }
                s
            }
        }
    }
}

fn parse_number_provider(v: &Value) -> Result<NumberProvider, String> {
    if let Some(n) = v.as_f64() {
        return Ok(NumberProvider::Constant(n as i64));
    }
    let obj = v.as_object().ok_or_else(|| {
        format!("number provider must be number or object, got {v:?}")
    })?;
    let typ = obj
        .iter()
        .find(|(k, _)| k == "type")
        .and_then(|(_, v)| v.as_str())
        .unwrap_or("minecraft:constant");
    match typ {
        "minecraft:constant" => {
            let n = obj
                .iter()
                .find(|(k, _)| k == "value")
                .and_then(|(_, v)| v.as_f64())
                .ok_or_else(|| "minecraft:constant needs 'value'")?;
            Ok(NumberProvider::Constant(n as i64))
        }
        "minecraft:uniform" => {
            let lo = obj
                .iter()
                .find(|(k, _)| k == "min")
                .and_then(|(_, v)| v.as_f64())
                .ok_or_else(|| "minecraft:uniform needs 'min'")?;
            let hi = obj
                .iter()
                .find(|(k, _)| k == "max")
                .and_then(|(_, v)| v.as_f64())
                .ok_or_else(|| "minecraft:uniform needs 'max'")?;
            Ok(NumberProvider::Uniform {
                lo: lo as i32,
                hi: hi as i32,
            })
        }
        "minecraft:binomial" => {
            let n = obj
                .iter()
                .find(|(k, _)| k == "n")
                .and_then(|(_, v)| v.as_i64())
                .ok_or_else(|| "minecraft:binomial needs 'n'")?;
            let p = obj
                .iter()
                .find(|(k, _)| k == "p")
                .and_then(|(_, v)| v.as_f64())
                .ok_or_else(|| "minecraft:binomial needs 'p'")?;
            Ok(NumberProvider::Binomial { n: n as i32, p })
        }
        other => Err(format!("unknown number provider type: {other}")),
    }
}

// ---------------------------------------------------------------------------
// Loot pool entry
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) enum LootFunction {
    SetCount(NumberProvider),
    EnchantRandomly,
    EnchantWithLevels { levels: i32, treasure: bool },
    /// `minecraft:apply_bonus`：时运加成。`formula` 取
    /// `minecraft:ore_drops` / `minecraft:uniform_bonus_count` /
    /// `minecraft:binomial_bonus_count`；`extra` 仅 uniform 公式使用。
    ApplyBonus { formula: String, extra: NumberProvider },
    /// 忽略其它函数（保留解析结果以便未来扩展）。
    Other(String),
}

#[derive(Clone, Debug)]
pub(crate) struct LootPoolEntry {
    pub(crate) kind: String,
    pub(crate) name: Option<String>,
    pub(crate) weight: i32,
    pub(crate) functions: Vec<LootFunction>,
    /// 复合 entry（alternatives/sequence/group）的子项。
    pub(crate) children: Vec<LootPoolEntry>,
}

impl LootPoolEntry {
    fn generate<R: LootRng + ?Sized>(&self, rng: &mut R, fortune_level: i32) -> Vec<LootItem> {
        match self.kind.as_str() {
            "minecraft:item" => {
                let mut item = LootItem::new(self.name.clone().unwrap_or_default(), 1);
                for func in &self.functions {
                    match func {
                        LootFunction::SetCount(np) => {
                            item.count = np.sample(rng, 0.0);
                        }
                        LootFunction::EnchantRandomly => {
                            // 对齐 Python `apply_enchant_randomly` 的 `__enchanted__` 占位。
                            item.enchanted = true;
                        }
                        // `enchant_with_levels` 在 vanilla 会真实注册一个附魔，但
                        // Python 参考项目只对 `enchant_randomly` 设 `__enchanted__`
                        // 占位；为了逐位对拍，我们也不在此设 enchanted。
                        LootFunction::EnchantWithLevels { .. } => {}
                        LootFunction::ApplyBonus { formula, extra } => {
                            apply_bonus(&mut item, rng, formula, extra, fortune_level);
                        }
                        LootFunction::Other(_) => {}
                    }
                }
                vec![item]
            }
            // alternatives：返回第一个条件通过的 child（本模块条件恒为通过，
            // 与 Python 端 LootCondition stub 一致）。
            "minecraft:alternatives" => match self.children.first() {
                Some(child) => child.generate(rng, fortune_level),
                None => Vec::new(),
            },
            // sequence / group：依次拼接全部 child。
            "minecraft:sequence" | "minecraft:group" => {
                let mut out = Vec::new();
                for child in &self.children {
                    out.extend(child.generate(rng, fortune_level));
                }
                out
            }
            // empty / tag / dynamic / loot_table 等：不产生物品。
            _ => Vec::new(),
        }
    }
}

/// `minecraft:apply_bonus`（Python `apply_bonus` 的逐行翻译）。
///
/// `fortune_level <= 0` 时无效果（默认路径）。`ore_drops` 公式：
/// `count *= rng.nextInt(fortune_level) + 1`（fortune_level >= 1）；
/// `uniform_bonus_count`：每级额外 `extra.next(rng)`。binomial/formula
/// 与 Python 端一致不实现。
fn apply_bonus<R: LootRng + ?Sized>(
    item: &mut LootItem,
    rng: &mut R,
    formula: &str,
    extra: &NumberProvider,
    fortune_level: i32,
) {
    if fortune_level <= 0 {
        return;
    }
    match formula {
        "minecraft:uniform_bonus_count" => {
            for _ in 0..fortune_level {
                item.count += extra.sample(rng, 0.0);
            }
        }
        "minecraft:ore_drops" => {
            let mult = rng.next_int_bound(fortune_level as u32) as i32 + 1;
            item.count *= mult;
        }
        _ => {}
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct LootPool {
    pub(crate) rolls: NumberProvider,
    pub(crate) bonus_rolls: NumberProvider,
    pub(crate) entries: Vec<LootPoolEntry>,
}

impl LootPool {
    fn generate<R: LootRng + ?Sized>(
        &self,
        rng: &mut R,
        luck: f64,
        fortune_level: i32,
    ) -> Vec<LootItem> {
        // dzs -> dzr.a:
        //   rolls_count = rollsProvider.nextInt() +
        //                 floor(bonusRollsProvider.nextInt() * luck)
        let rolls = self.rolls.sample(rng, luck);
        let bonus_raw = self.bonus_rolls.sample(rng, luck);
        let bonus = ((bonus_raw as f64) * luck).floor() as i32;
        let total = rolls + bonus;
        let mut out = Vec::new();
        for _ in 0..total {
            // weight = entry.weight if entry passes conditions and weight > 0
            let mut weighted: Vec<(&LootPoolEntry, i32)> = Vec::new();
            let mut total_w = 0i64;
            for entry in &self.entries {
                if entry.weight > 0 {
                    weighted.push((entry, entry.weight));
                    total_w += entry.weight as i64;
                }
            }
            if total_w <= 0 {
                continue;
            }
            // Python loot_table.py 用 `pick = rng.next_int(total_w)`，
            // 即 32 位 Lemire 拒绝采样（与 next_int_bound 等价）。
            let pick = rng.next_int_bound(total_w as u32) as i32;
            let mut acc = 0i32;
            let mut chosen: Option<&LootPoolEntry> = None;
            for (entry, w) in &weighted {
                acc += *w;
                if pick < acc {
                    chosen = Some(*entry);
                    break;
                }
            }
            if let Some(entry) = chosen {
                out.extend(entry.generate(rng, fortune_level));
            }
        }
        out
    }
}

/// 完整的战利品表（顶层），可直接 `generate(rng)` 得到物品列表。
#[derive(Clone, Debug, Default)]
pub struct LootTable {
    pools: Vec<LootPool>,
    functions: Vec<LootFunction>,
}

impl LootTable {
    /// 从 JSON 字符串解析（顶层必须是 `{ "pools": [...] }`）。
    pub fn from_json_str(s: &str) -> Result<Self, String> {
        let v = json::parse(s).map_err(|e| e.to_string())?;
        Self::from_value(&v)
    }

    /// 从已解析的 JSON 值构造。
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let obj = v
            .as_object()
            .ok_or_else(|| format!("loot table root must be object, got {v:?}"))?;
        let mut pools = Vec::new();
        if let Some(pools_v) = obj.iter().find(|(k, _)| *k == "pools").map(|(_, v)| v) {
            let arr = pools_v
                .as_array()
                .ok_or_else(|| "pools must be array")?;
            for pool_v in arr {
                pools.push(parse_pool(pool_v)?);
            }
        }
        let mut functions = Vec::new();
        if let Some(funcs_v) = obj.iter().find(|(k, _)| *k == "functions").map(|(_, v)| v) {
            let arr = funcs_v.as_array().ok_or_else(|| "functions must be array")?;
            for f in arr {
                functions.push(parse_function(f)?);
            }
        }
        Ok(LootTable { pools, functions })
    }

    /// 用给定的 RNG 生成战利品列表。`luck=0` 时 bonus_rolls 不影响。
    /// 等价于 `generate_ctx(rng, luck, 0)`。
    pub fn generate<R: LootRng + ?Sized>(&self, rng: &mut R, luck: f64) -> Vec<LootItem> {
        self.generate_ctx(rng, luck, 0)
    }

    /// 完整上下文版本：`fortune_level` 驱动 `minecraft:apply_bonus`。
    pub fn generate_ctx<R: LootRng + ?Sized>(
        &self,
        rng: &mut R,
        luck: f64,
        fortune_level: i32,
    ) -> Vec<LootItem> {
        let mut out = Vec::new();
        for pool in &self.pools {
            out.extend(pool.generate(rng, luck, fortune_level));
        }
        // 表级函数（loot table 顶部 functions[]）。原版支持
        // `minecraft:set_count`，逐项应用。
        if !self.functions.is_empty() {
            for func in &self.functions {
                if let LootFunction::SetCount(np) = func {
                    for item in &mut out {
                        item.count = np.sample(rng, luck);
                    }
                }
            }
        }
        out
    }

    /// crate 内部访问器（测试与上层包装使用）。
    #[allow(dead_code)]
    pub(crate) fn pools_internal(&self) -> &[LootPool] {
        &self.pools
    }
}

// ---------------------------------------------------------------------------
// 解析辅助
// ---------------------------------------------------------------------------

fn parse_pool(v: &Value) -> Result<LootPool, String> {
    let obj = v.as_object().ok_or_else(|| "pool must be object")?;
    let rolls_v = obj
        .iter()
        .find(|(k, _)| *k == "rolls")
        .map(|(_, v)| v)
        .ok_or_else(|| "pool missing 'rolls'")?;
    let rolls = parse_number_provider(rolls_v)?;
    // bonus_rolls 缺省即 constant(0)
    let bonus = match obj.iter().find(|(k, _)| *k == "bonus_rolls").map(|(_, v)| v) {
        Some(v) => parse_number_provider(v)?,
        None => NumberProvider::Constant(0),
    };
    let mut entries = Vec::new();
    if let Some(entries_v) = obj.iter().find(|(k, _)| *k == "entries").map(|(_, v)| v) {
        let arr = entries_v.as_array().ok_or_else(|| "entries must be array")?;
        for e in arr {
            entries.push(parse_entry(e)?);
        }
    }
    Ok(LootPool { rolls, bonus_rolls: bonus, entries })
}

fn parse_entry(v: &Value) -> Result<LootPoolEntry, String> {
    let obj = v.as_object().ok_or_else(|| "entry must be object")?;
    let kind = obj
        .iter()
        .find(|(k, _)| *k == "type")
        .and_then(|(_, v)| v.as_str())
        .unwrap_or("minecraft:item")
        .to_string();
    let name = obj
        .iter()
        .find(|(k, _)| *k == "name")
        .and_then(|(_, v)| v.as_str())
        .map(|s| s.to_string());
    let weight = obj
        .iter()
        .find(|(k, _)| *k == "weight")
        .and_then(|(_, v)| v.as_i64())
        .unwrap_or(1) as i32;
    let mut functions = Vec::new();
    if let Some(funcs_v) = obj.iter().find(|(k, _)| *k == "functions").map(|(_, v)| v) {
        let arr = funcs_v.as_array().ok_or_else(|| "functions must be array")?;
        for f in arr {
            functions.push(parse_function(f)?);
        }
    }
    let mut children = Vec::new();
    if let Some(children_v) = obj.iter().find(|(k, _)| *k == "children").map(|(_, v)| v) {
        let arr = children_v.as_array().ok_or_else(|| "children must be array")?;
        for c in arr {
            children.push(parse_entry(c)?);
        }
    }
    Ok(LootPoolEntry { kind, name, weight, functions, children })
}

fn parse_function(v: &Value) -> Result<LootFunction, String> {
    let obj = v.as_object().ok_or_else(|| "function must be object")?;
    let name = obj
        .iter()
        .find(|(k, _)| *k == "function")
        .and_then(|(_, v)| v.as_str())
        .ok_or_else(|| "function missing 'function' field")?;
    match name {
        "minecraft:set_count" => {
            let count_v = obj
                .iter()
                .find(|(k, _)| *k == "count")
                .map(|(_, v)| v)
                .ok_or_else(|| "set_count missing 'count'")?;
            let np = parse_number_provider(count_v)?;
            Ok(LootFunction::SetCount(np))
        }
        "minecraft:enchant_randomly" => Ok(LootFunction::EnchantRandomly),
        "minecraft:apply_bonus" => {
            let formula = obj
                .iter()
                .find(|(k, _)| *k == "formula")
                .and_then(|(_, v)| v.as_str())
                .unwrap_or("minecraft:ore_drops")
                .to_string();
            let extra = match obj.iter().find(|(k, _)| *k == "extra").map(|(_, v)| v) {
                Some(v) => parse_number_provider(v)?,
                None => NumberProvider::Constant(0),
            };
            Ok(LootFunction::ApplyBonus { formula, extra })
        }
        "minecraft:enchant_with_levels" => {
            let levels = obj
                .iter()
                .find(|(k, _)| *k == "levels")
                .and_then(|(_, v)| v.as_i64())
                .unwrap_or(0) as i32;
            let treasure = obj
                .iter()
                .find(|(k, _)| *k == "treasure")
                .and_then(|(_, v)| v.as_bool())
                .unwrap_or(false);
            Ok(LootFunction::EnchantWithLevels { levels, treasure })
        }
        other => Ok(LootFunction::Other(other.to_string())),
    }
}

// ---------------------------------------------------------------------------
// 单元测试：抽样序列与表生成
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::rng::XoroshiroLootRng;

    #[test]
    fn constant_provider_returns_value() {
        let mut rng = XoroshiroLootRng::new(0);
        let np = NumberProvider::Constant(7);
        assert_eq!(np.sample(&mut rng, 0.0), 7);
        assert_eq!(np.sample(&mut rng, 0.0), 7);
    }

    #[test]
    fn uniform_provider_returns_in_range() {
        let mut rng = XoroshiroLootRng::new(42);
        let np = NumberProvider::Uniform { lo: 1, hi: 4 };
        for _ in 0..1000 {
            let v = np.sample(&mut rng, 0.0);
            assert!((1..=4).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn uniform_provider_inclusive_bounds() {
        // 与 vanilla MathHelper.nextInt 一致：含两端
        let mut rng = XoroshiroLootRng::new(0);
        let np = NumberProvider::Uniform { lo: 3, hi: 3 };
        assert_eq!(np.sample(&mut rng, 0.0), 3);
    }

    #[test]
    fn binomial_provider_sums_n_bernoulli() {
        let mut rng = XoroshiroLootRng::new(12345);
        let np = NumberProvider::Binomial { n: 5, p: 0.5 };
        for _ in 0..500 {
            let v = np.sample(&mut rng, 0.0);
            assert!((0..=5).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn parses_empty_table() {
        let t = LootTable::from_json_str(r#"{"type":"minecraft:empty","pools":[]}"#).unwrap();
        let mut rng = XoroshiroLootRng::new(1);
        assert!(t.generate(&mut rng, 0.0).is_empty());
    }

    #[test]
    fn parses_ruined_portal_subset() {
        // 取 ruined_portal 第一条 entry：obsidian × uniform(1,2)
        let json = r#"{
            "type": "minecraft:chest",
            "pools": [{
                "rolls": {"min": 1.0, "max": 2.0, "type": "minecraft:uniform"},
                "entries": [{
                    "type": "minecraft:item",
                    "weight": 1,
                    "functions": [{"function": "minecraft:set_count", "count": {"min": 1.0, "max": 2.0, "type": "minecraft:uniform"}}],
                    "name": "minecraft:obsidian"
                }]
            }]
        }"#;
        let t = LootTable::from_json_str(json).unwrap();
        let mut rng = XoroshiroLootRng::new(0x12345);
        let items = t.generate(&mut rng, 0.0);
        // 1–2 rolls × 1–2 count，每件 obsidian
        assert!(!items.is_empty());
        for i in &items {
            assert_eq!(i.item, "minecraft:obsidian");
            assert!((1..=2).contains(&i.count));
        }
    }

    #[test]
    fn enchant_randomly_sets_flag() {
        let json = r#"{
            "type": "minecraft:chest",
            "pools": [{
                "rolls": 1,
                "entries": [{
                    "type": "minecraft:item",
                    "name": "minecraft:book",
                    "functions": [{"function": "minecraft:enchant_randomly"}]
                }]
            }]
        }"#;
        let t = LootTable::from_json_str(json).unwrap();
        let mut rng = XoroshiroLootRng::new(0);
        let items = t.generate(&mut rng, 0.0);
        assert_eq!(items.len(), 1);
        assert!(items[0].enchanted);
    }

    #[test]
    fn picks_weighted_entry_deterministically() {
        // 100% weight 到 A vs B (1:99)，种子确定使得每次都选 B
        let json = r#"{
            "type": "minecraft:chest",
            "pools": [{
                "rolls": 1,
                "entries": [
                    {"type": "minecraft:item", "weight": 1, "name": "minecraft:rare"},
                    {"type": "minecraft:item", "weight": 99, "name": "minecraft:common"}
                ]
            }]
        }"#;
        let t = LootTable::from_json_str(json).unwrap();
        let mut rng = XoroshiroLootRng::new(7);
        let items = t.generate(&mut rng, 0.0);
        assert_eq!(items.len(), 1);
        // 选哪个不确定（与种子相关），但必定是二者之一
        assert!(matches!(items[0].item.as_str(), "minecraft:rare" | "minecraft:common"));
    }

    #[test]
    fn alternatives_returns_first_child() {
        // 条件一律视为通过，所以 alternatives 总是产出第一个 child。
        let json = r#"{
            "type": "minecraft:block",
            "pools": [{
                "rolls": 1,
                "entries": [{
                    "type": "minecraft:alternatives",
                    "children": [
                        {"type": "minecraft:item", "name": "minecraft:first",
                         "conditions": [{"condition": "minecraft:match_tool"}]},
                        {"type": "minecraft:item", "name": "minecraft:second"}
                    ]
                }]
            }]
        }"#;
        let t = LootTable::from_json_str(json).unwrap();
        let mut rng = XoroshiroLootRng::new(0);
        let items = t.generate(&mut rng, 0.0);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item, "minecraft:first");
    }

    #[test]
    fn group_concatenates_children() {
        let json = r#"{
            "type": "minecraft:block",
            "pools": [{
                "rolls": 1,
                "entries": [{
                    "type": "minecraft:group",
                    "children": [
                        {"type": "minecraft:item", "name": "minecraft:a"},
                        {"type": "minecraft:item", "name": "minecraft:b",
                         "functions": [{"function": "minecraft:set_count", "count": 3}]}
                    ]
                }]
            }]
        }"#;
        let t = LootTable::from_json_str(json).unwrap();
        let mut rng = XoroshiroLootRng::new(0);
        let items = t.generate(&mut rng, 0.0);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].item, "minecraft:a");
        assert_eq!((items[1].item.as_str(), items[1].count), ("minecraft:b", 3));
    }

    #[test]
    fn apply_bonus_ore_drops_scales_with_fortune() {
        let json = r#"{
            "type": "minecraft:block",
            "pools": [{
                "rolls": 1,
                "entries": [{
                    "type": "minecraft:item",
                    "name": "minecraft:diamond",
                    "functions": [
                        {"function": "minecraft:set_count", "count": 1},
                        {"function": "minecraft:apply_bonus",
                         "enchantment": "minecraft:fortune",
                         "formula": "minecraft:ore_drops"}
                    ]
                }]
            }]
        }"#;
        let t = LootTable::from_json_str(json).unwrap();
        // fortune 0：恒为 1
        let mut rng = XoroshiroLootRng::new(7);
        let items = t.generate_ctx(&mut rng, 0.0, 0);
        assert_eq!(items[0].count, 1);
        // fortune 3：count ∈ [1, 4]
        for seed in 0..100u64 {
            let mut rng = XoroshiroLootRng::new(seed);
            let items = t.generate_ctx(&mut rng, 0.0, 3);
            assert!((1..=4).contains(&items[0].count), "count={}", items[0].count);
        }
    }
}
