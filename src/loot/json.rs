//! 零依赖的迷你 JSON 解析器，仅支持 loot table 实际用到的子集：
//! 对象、数组、字符串、数字（IEEE-754 f64）、`true`/`false`/`null`。
//!
//! 设计目标：
//! - 单一文件、无外部依赖（满足 `minecraft_seed_core` 的零依赖约束）。
//! - 错误位置精确到行列，方便排查格式异常的 JSON。
//! - 解析后保留 f64 数字（loot table 只用到整数即可，因此整数读取端再做
//!   截断/四舍五入以匹配 Minecraft 的 JSON 解析语义）。
//!
//! 设计参考 Minecraft 1.20.1 的 `com.google.gson` 行为：
//! - 整数（无小数点/无指数）按 double 解析后取整时若超出 i64 范围，
//!   会丢失精度；本解析器在解析阶段保留原始 `f64` 位模式，调用方用
//!   `as_i64()` 取整时再显式选择向下/四舍五入/截断。
//! - 字符串支持 `\uXXXX` 与常见转义；其它转义保留为原样（与 gson 不同，
//!   但 loot table 不依赖）。
//!
//! 不支持的 JSON 语法（与 loot table 无关，调用方需注意）：
//! - 注释（JSON 不允许）
//! - 多行字符串
//! - 超大整数（> 2^53 会丢精度）

use std::fmt;

/// JSON 值。
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        if let Value::Object(items) = self {
            Some(items)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        if let Value::Array(items) = self {
            Some(items)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let Value::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// 整数（向下取整）。Minecraft 的 JSON 解析对整数直接 cast 到 long，
    /// 这里用 `as i64`（即向零截断）等价语义。
    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|n| n as i64)
    }

    /// 在对象里按 key 取值（O(n) 线性扫描，loot table 字段极少，可接受）。
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_object()
            .and_then(|items| items.iter().find(|(k, _)| k == key).map(|(_, v)| v))
    }
}

/// JSON 解析错误（含行/列位置）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub col: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON parse error at {}:{}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}

/// 解析整个字符串为 [`Value`]。
pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut p = Parser::new(input);
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if !p.is_eof() {
        return Err(p.error("trailing data after JSON value"));
    }
    Ok(v)
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser {
            bytes: input.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            line: self.line,
            col: self.col,
            message: msg.into(),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.bump();
                }
                _ => break,
            }
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), ParseError> {
        match self.peek() {
            Some(b) if b == c => {
                self.bump();
                Ok(())
            }
            Some(b) => Err(self.error(format!("expected '{}', got '{}'", c as char, b as char))),
            None => Err(self.error(format!("expected '{}', got EOF", c as char))),
        }
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Value::String),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(c) => Err(self.error(format!("unexpected character '{}'", c as char))),
            None => Err(self.error("unexpected EOF")),
        }
    }

    fn parse_object(&mut self) -> Result<Value, ParseError> {
        self.expect(b'{')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(Value::Object(items));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let value = self.parse_value()?;
            items.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                }
                Some(b'}') => {
                    self.bump();
                    return Ok(Value::Object(items));
                }
                Some(c) => return Err(self.error(format!("expected ',' or '}}', got '{}'", c as char))),
                None => return Err(self.error("unterminated object")),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(Value::Array(items));
        }
        loop {
            let v = self.parse_value()?;
            items.push(v);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.bump();
                }
                Some(b']') => {
                    self.bump();
                    return Ok(Value::Array(items));
                }
                Some(c) => return Err(self.error(format!("expected ',' or ']', got '{}'", c as char))),
                None => return Err(self.error("unterminated array")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                Some(b'"') => return Ok(out),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'b') => out.push('\u{08}'),
                    Some(b'f') => out.push('\u{0C}'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'u') => {
                        let cp = self.parse_hex4()?;
                        if (0xD800..=0xDBFF).contains(&cp) {
                            // high surrogate; expect low surrogate next
                            match (self.peek(), self.bump()) {
                                (Some(b'\\'), _) => {
                                    self.bump(); // 'u'
                                    let lo = self.parse_hex4()?;
                                    if !(0xDC00..=0xDFFF).contains(&lo) {
                                        return Err(self.error("invalid low surrogate"));
                                    }
                                    let combined = 0x1_0000
                                        + (((cp - 0xD800) << 10) | (lo - 0xDC00));
                                    if let Some(ch) = char::from_u32(combined) {
                                        out.push(ch);
                                    }
                                }
                                _ => return Err(self.error("expected low surrogate")),
                            }
                        } else if let Some(ch) = char::from_u32(cp) {
                            out.push(ch);
                        } else {
                            return Err(self.error(format!("invalid Unicode escape \\u{cp:04X}")));
                        }
                    }
                    Some(c) => return Err(self.error(format!("invalid escape '\\{}'", c as char))),
                    None => return Err(self.error("unterminated string escape")),
                },
                Some(c) if c < 0x20 => {
                    return Err(self.error(format!(
                        "control character 0x{:02x} in string",
                        c
                    )))
                }
                Some(c) => out.push(c as char),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32, ParseError> {
        let mut v: u32 = 0;
        for _ in 0..4 {
            let b = self.bump().ok_or_else(|| self.error("unterminated \\u escape"))?;
            v = v << 4
                | match b {
                    b'0'..=b'9' => (b - b'0') as u32,
                    b'a'..=b'f' => (b - b'a' + 10) as u32,
                    b'A'..=b'F' => (b - b'A' + 10) as u32,
                    _ => return Err(self.error(format!("invalid hex digit '{}'", b as char))),
                };
        }
        Ok(v)
    }

    fn parse_bool(&mut self) -> Result<Value, ParseError> {
        if self.try_consume_literal(b"true") {
            Ok(Value::Bool(true))
        } else if self.try_consume_literal(b"false") {
            Ok(Value::Bool(false))
        } else {
            Err(self.error("invalid boolean literal"))
        }
    }

    fn parse_null(&mut self) -> Result<Value, ParseError> {
        if self.try_consume_literal(b"null") {
            Ok(Value::Null)
        } else {
            Err(self.error("invalid null literal"))
        }
    }

    fn try_consume_literal(&mut self, lit: &[u8]) -> bool {
        if self.bytes[self.pos..].starts_with(lit) {
            for _ in 0..lit.len() {
                self.bump();
            }
            true
        } else {
            false
        }
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.bump();
        }
        // 整数部分：0 单独 OR 1–9 后跟 0–9*
        match self.peek() {
            Some(b'0') => {
                self.bump();
            }
            Some(b'1'..=b'9') => {
                while let Some(b'0'..=b'9') = self.peek() {
                    self.bump();
                }
            }
            Some(c) => return Err(self.error(format!("invalid number digit '{}'", c as char))),
            None => return Err(self.error("unterminated number")),
        }
        // 小数部分
        if self.peek() == Some(b'.') {
            self.bump();
            let frac_start = self.pos;
            while let Some(b'0'..=b'9') = self.peek() {
                self.bump();
            }
            if self.pos == frac_start {
                return Err(self.error("missing digits after '.'"));
            }
        }
        // 指数
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.bump();
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.bump();
            }
            let exp_start = self.pos;
            while let Some(b'0'..=b'9') = self.peek() {
                self.bump();
            }
            if self.pos == exp_start {
                return Err(self.error("missing digits in exponent"));
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.error("invalid UTF-8 in number"))?;
        let n: f64 = s
            .parse()
            .map_err(|_| self.error(format!("invalid number literal '{s}'")))?;
        Ok(Value::Number(n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> Value {
        parse(s).expect("must parse")
    }

    #[test]
    fn parses_empty_object_and_array() {
        assert_eq!(parse_ok("{}"), Value::Object(vec![]));
        assert_eq!(parse_ok("[]"), Value::Array(vec![]));
    }

    #[test]
    fn parses_whitespace_and_unicode_escapes() {
        let v = parse_ok(r#"{ "a" : "\u00e9\u0041", "b" : "\uD83D\uDE00" }"#);
        let obj = v.as_object().unwrap();
        assert_eq!(obj[0].1.as_str(), Some("éA"));
        assert_eq!(obj[1].1.as_str(), Some("😀"));
    }

    #[test]
    fn parses_numbers_including_negative_and_exponent() {
        let v = parse_ok("[-1, 0, 1.5, 2e3, -3.14e-2]");
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0].as_f64(), Some(-1.0));
        assert_eq!(arr[1].as_f64(), Some(0.0));
        assert_eq!(arr[2].as_f64(), Some(1.5));
        assert_eq!(arr[3].as_f64(), Some(2000.0));
        assert!((arr[4].as_f64().unwrap() + 0.0314).abs() < 1e-9);
    }

    #[test]
    fn parses_bools_and_null() {
        let v = parse_ok("[true, false, null]");
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0].as_bool(), Some(true));
        assert_eq!(arr[1].as_bool(), Some(false));
        assert_eq!(arr[2], Value::Null);
    }

    #[test]
    fn object_field_lookup() {
        let v = parse_ok(r#"{"name": "minecraft:stone", "weight": 3}"#);
        assert_eq!(v.get("name").and_then(|v| v.as_str()), Some("minecraft:stone"));
        assert_eq!(v.get("weight").and_then(|v| v.as_i64()), Some(3));
        assert_eq!(v.get("missing"), None);
    }

    #[test]
    fn nested_structures() {
        let v = parse_ok(
            r#"{
                "pools": [
                    {"rolls": {"min": 1.0, "max": 4.0, "type": "minecraft:uniform"},
                     "entries": [{"type": "minecraft:item", "weight": 5}]}
                ]
            }"#,
        );
        let pools = v.get("pools").and_then(|v| v.as_array()).unwrap();
        assert_eq!(pools.len(), 1);
        let roll = pools[0].get("rolls").unwrap();
        assert_eq!(roll.get("type").and_then(|v| v.as_str()), Some("minecraft:uniform"));
        assert_eq!(roll.get("min").and_then(|v| v.as_f64()), Some(1.0));
        assert_eq!(roll.get("max").and_then(|v| v.as_f64()), Some(4.0));
    }

    #[test]
    fn trailing_data_errors() {
        let err = parse("{\"a\":1} junk").unwrap_err();
        assert!(err.message.contains("trailing"));
    }

    #[test]
    fn unterminated_string_errors_with_position() {
        let err = parse("[\"unterminated]").unwrap_err();
        assert!(err.line >= 1);
    }

    #[test]
    fn missing_object_key_quote_errors() {
        let err = parse("{a:1}").unwrap_err();
        assert!(err.message.contains("expected"));
    }

    #[test]
    fn control_char_in_string_rejected() {
        let err = parse("\"a\nb\"").unwrap_err();
        assert!(err.message.contains("control"));
    }
}
