//! Java `String` 的 UTF-16 值语义。

use std::cmp::Ordering;
use std::fmt;

/// 保存 Java `String` 的原始 UTF-16 code unit。
///
/// Rust [`String`] 只能保存合法 UTF-8，无法表示 Java
/// `"\u{1F600}".substring(0, 1)` 产生的未配对高代理项。本类型以
/// `Vec<u16>` 保存字符串，从而让 `length`、`charAt`、`substring`、
/// 比较与连接继续遵循 Java 语义。
///
/// 对应 Java: `java.lang.String`。
#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct JavaString {
    units: Vec<u16>,
    utf8: Option<String>,
}

impl JavaString {
    /// 从原始 UTF-16 code unit 创建 Java 字符串。
    ///
    /// # 参数
    ///
    /// - `units`：需要原样保存的 UTF-16 code unit。
    ///
    /// # 返回值
    ///
    /// 返回可表示未配对代理项的 Java 字符串。
    /// 对应 Java：`java.lang.String` 的 UTF-16 code unit 存储语义。
    pub fn from_utf16_units(units: Vec<u16>) -> Self {
        let utf8 = String::from_utf16(&units).ok();
        Self { units, utf8 }
    }

    /// 返回原始 UTF-16 code unit。
    ///
    /// # 返回值
    ///
    /// 返回与 Java `String#charAt` 所观察到的 code unit 序列一致的切片。
    /// 对应 Java：`java.lang.String#charAt(int)` 可观察到的 UTF-16 序列。
    pub fn utf16_units(&self) -> &[u16] {
        &self.units
    }

    /// 返回按值迭代的 UTF-16 code unit。
    ///
    /// # 返回值
    ///
    /// 返回与 Rust `str::encode_utf16` 形态一致、但不会丢失未配对代理项的迭代器。
    /// 对应 Java：无（Rust 对 Java UTF-16 存储的迭代适配）。
    pub fn encode_utf16(&self) -> impl Iterator<Item = u16> + '_ {
        self.units.iter().copied()
    }

    /// 返回 UTF-16 code unit 数量。对应 Java `String#length()`。
    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// 判断字符串是否没有 UTF-16 code unit。对应 Java `String#isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// 在值可无损表示为 UTF-8 时返回 Rust 字符串切片。
    ///
    /// # 返回值
    ///
    /// 合法 Unicode 字符串返回 `Some`；包含未配对代理项时返回 `None`。
    /// 对应 Java：无（Rust UTF-8 宿主边界适配）。
    pub fn as_str(&self) -> Option<&str> {
        self.utf8.as_deref()
    }

    /// 将值无损转换为 Rust [`String`]。
    ///
    /// # 返回值
    ///
    /// 合法 Unicode 字符串返回 `Some`；包含未配对代理项时返回 `None`。
    /// 对应 Java：无（Rust UTF-8 宿主边界适配）。
    pub fn to_rust_string(&self) -> Option<String> {
        self.utf8.clone()
    }

    /// 为日志或 UTF-8 宿主边界生成替换非法代理项后的文本。
    ///
    /// 该方法不得用于引擎内部相等、比较或索引语义。
    /// 对应 Java：无（Rust 日志与 UTF-8 宿主边界的有损适配）。
    pub fn to_string_lossy(&self) -> String {
        self.utf8
            .clone()
            .unwrap_or_else(|| String::from_utf16_lossy(&self.units))
    }

    /// 读取指定 UTF-16 下标的 code unit。对应 Java `String#charAt(int)`。
    pub fn char_at(&self, index: usize) -> Option<u16> {
        self.units.get(index).copied()
    }

    /// 判断是否包含指定 Java 字符串。对应 Java `String#contains`。
    pub fn contains(&self, needle: &Self) -> bool {
        if needle.is_empty() {
            return true;
        }
        self.units
            .windows(needle.units.len())
            .any(|candidate| candidate == needle.units.as_slice())
    }

    /// 判断是否从指定 UTF-16 下标开始匹配前缀。
    /// 对应 Java `String#startsWith(String, int)`。
    pub fn starts_with_at(&self, prefix: &Self, offset: i64) -> bool {
        usize::try_from(offset)
            .ok()
            .and_then(|offset| {
                self.units
                    .get(offset..offset.saturating_add(prefix.units.len()))
            })
            .is_some_and(|candidate| candidate == prefix.units.as_slice())
    }

    /// 判断是否匹配前缀。对应 Java `String#startsWith(String)`。
    pub fn starts_with(&self, prefix: &Self) -> bool {
        self.starts_with_at(prefix, 0)
    }

    /// 判断是否匹配后缀。对应 Java `String#endsWith(String)`。
    pub fn ends_with(&self, suffix: &Self) -> bool {
        self.units.ends_with(&suffix.units)
    }

    /// 从指定 UTF-16 下标查找子串。对应 Java `String#indexOf(String, int)`。
    pub fn index_of(&self, needle: &Self, from_index: i64) -> i32 {
        let start = usize::try_from(from_index.max(0))
            .unwrap_or(usize::MAX)
            .min(self.units.len());
        if needle.is_empty() {
            return start as i32;
        }
        self.units[start..]
            .windows(needle.units.len())
            .position(|candidate| candidate == needle.units.as_slice())
            .map(|offset| (start + offset) as i32)
            .unwrap_or(-1)
    }

    /// 按 UTF-16 code unit 字典序比较。对应 Java `String#compareTo`。
    pub fn compare_to(&self, other: &Self) -> i32 {
        for (left, right) in self.units.iter().zip(other.units.iter()) {
            if left != right {
                return i32::from(*left) - i32::from(*right);
            }
        }
        self.units.len() as i32 - other.units.len() as i32
    }

    /// 截取 UTF-16 区间。对应 Java `String#substring(int, int)`。
    pub fn substring(&self, begin: usize, end: usize) -> Option<Self> {
        (begin <= end && end <= self.units.len())
            .then(|| Self::from_utf16_units(self.units[begin..end].to_vec()))
    }

    /// 连接两个 Java 字符串，保留未配对代理项。
    /// 对应 Java：`java.lang.String#concat(String)`。
    pub fn concat(&self, other: &Self) -> Self {
        let mut units = Vec::with_capacity(self.units.len().saturating_add(other.units.len()));
        units.extend_from_slice(&self.units);
        units.extend_from_slice(&other.units);
        Self::from_utf16_units(units)
    }

    /// 替换所有非重叠目标序列。对应 Java `String#replace(CharSequence, CharSequence)`。
    pub fn replace(&self, from: &Self, to: &Self) -> Self {
        if from.is_empty() {
            let mut units = Vec::with_capacity(
                self.units
                    .len()
                    .saturating_add(self.units.len().saturating_add(1) * to.units.len()),
            );
            units.extend_from_slice(&to.units);
            for unit in &self.units {
                units.push(*unit);
                units.extend_from_slice(&to.units);
            }
            return Self::from_utf16_units(units);
        }

        let mut units = Vec::new();
        let mut index = 0usize;
        while index < self.units.len() {
            if self.units[index..].starts_with(&from.units) {
                units.extend_from_slice(&to.units);
                index += from.units.len();
            } else {
                units.push(self.units[index]);
                index += 1;
            }
        }
        Self::from_utf16_units(units)
    }

    /// 删除两端不大于 U+0020 的 code unit。对应 Java `String#trim()`。
    pub fn trim(&self) -> Self {
        let start = self
            .units
            .iter()
            .position(|unit| *unit > 0x20)
            .unwrap_or(self.units.len());
        let end = self
            .units
            .iter()
            .rposition(|unit| *unit > 0x20)
            .map(|index| index + 1)
            .unwrap_or(start);
        Self::from_utf16_units(self.units[start..end].to_vec())
    }

    /// 使用当前 Unicode 映射转换大写并保留未配对代理项。
    ///
    /// 合法 Unicode 输入沿用既有 Rust 映射；未配对代理项原样保留。
    /// 对应 Java：`java.lang.String#toUpperCase()`。
    pub fn to_uppercase(&self) -> Self {
        self.map_case(char::to_uppercase)
    }

    /// 使用当前 Unicode 映射转换小写并保留未配对代理项。
    ///
    /// 合法 Unicode 输入沿用既有 Rust 映射；未配对代理项原样保留。
    /// 对应 Java：`java.lang.String#toLowerCase()`。
    pub fn to_lowercase(&self) -> Self {
        self.map_case(char::to_lowercase)
    }

    fn map_case<I>(&self, map: impl Fn(char) -> I) -> Self
    where
        I: Iterator<Item = char>,
    {
        if let Some(value) = &self.utf8 {
            let mapped: String = value.chars().flat_map(map).collect();
            return mapped.into();
        }

        let mut units = Vec::with_capacity(self.units.len());
        for decoded in char::decode_utf16(self.units.iter().copied()) {
            match decoded {
                Ok(character) => {
                    for mapped in map(character) {
                        let mut buffer = [0u16; 2];
                        units.extend_from_slice(mapped.encode_utf16(&mut buffer));
                    }
                }
                Err(error) => units.push(error.unpaired_surrogate()),
            }
        }
        Self::from_utf16_units(units)
    }

    /// 计算 Java `String#hashCode()`。
    /// 对应 Java：`java.lang.String#hashCode()`。
    pub fn java_hash_code(&self) -> i32 {
        self.units.iter().fold(0i32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(i32::from(*unit))
        })
    }
}

impl From<String> for JavaString {
    fn from(value: String) -> Self {
        let units = value.encode_utf16().collect();
        Self {
            units,
            utf8: Some(value),
        }
    }
}

impl From<&str> for JavaString {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<&String> for JavaString {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl From<&&str> for JavaString {
    fn from(value: &&str) -> Self {
        (*value).into()
    }
}

impl From<Vec<u16>> for JavaString {
    fn from(value: Vec<u16>) -> Self {
        Self::from_utf16_units(value)
    }
}

impl Ord for JavaString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.units.cmp(&other.units)
    }
}

impl PartialOrd for JavaString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for JavaString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.utf8 {
            Some(value) => formatter.debug_tuple("JavaString").field(value).finish(),
            None => formatter
                .debug_struct("JavaString")
                .field("utf16_units", &self.units)
                .finish(),
        }
    }
}

impl fmt::Display for JavaString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::JavaString;

    #[test]
    fn preserves_unpaired_surrogate_substrings() {
        let emoji = JavaString::from("😀");
        let high = emoji.substring(0, 1).expect("valid UTF-16 range");
        let low = emoji.substring(1, 2).expect("valid UTF-16 range");

        assert_eq!(emoji.len(), 2);
        assert_eq!(high.utf16_units(), &[0xD83D]);
        assert_eq!(low.utf16_units(), &[0xDE00]);
        assert!(high.as_str().is_none());
        assert_eq!(high.concat(&low), emoji);
    }

    #[test]
    fn java_string_operations_use_code_units() {
        let value = JavaString::from("a😀b");
        let emoji = JavaString::from("😀");

        assert_eq!(value.len(), 4);
        assert_eq!(value.index_of(&emoji, 0), 1);
        assert!(value.starts_with_at(&emoji, 1));
        assert_eq!(value.char_at(1), Some(0xD83D));
        assert_eq!(JavaString::from("abc").java_hash_code(), 96_354);
    }
}
