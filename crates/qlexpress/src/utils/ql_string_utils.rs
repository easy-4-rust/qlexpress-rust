//! String escape parsing, mirroring Java `QLStringUtils`.

use crate::runtime::data::java_string::JavaString;

/// 按 QLExpress 字符串字面量规则解析引号和转义序列。
/// 对应 Java: `com.alibaba.qlexpress4.utils.QLStringUtils`。
pub struct QLStringUtils;

impl QLStringUtils {
    /// 构建或解析 string escape。
    /// 参数：`origin_str`；返回：保留 Java UTF-16 code unit 的字符串。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/QLStringUtils.java`，方法 `parseStringEscape`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `parseStringEscape`: strip the first and last character (the
    /// surrounding quotes) and resolve escape sequences.
    /// 对应 Java：`QLStringUtils#parseStringEscape(String)`。
    pub fn parse_string_escape(origin_str: &str) -> JavaString {
        let end = origin_str.encode_utf16().count() as i32 - 1;
        Self::parse_string_escape_start_end(origin_str, 1, end)
    }

    /// 构建或解析 string escape start end。
    /// 参数：`origin_str`、`start`、`end`；返回：保留 Java UTF-16 code unit 的字符串。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/QLStringUtils.java`，方法 `parseStringEscapeStartEnd`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `parseStringEscapeStartEnd`: resolve escapes in
    /// `origin_str[start..end]` (char indices).
    ///
    /// Recognized escapes: `\b \t \n \f \r \" \' \\ \$`. An unknown escape
    /// silently drops both characters (as in the Java switch with no
    /// default branch).
    /// 对应 Java: com.alibaba.qlexpress4.utils.QLStringUtils#parseStringEscapeStartEnd。
    pub fn parse_string_escape_start_end(origin_str: &str, start: i32, end: i32) -> JavaString {
        const INIT: u8 = 0;
        const ESCAPE: u8 = 1;

        let chars: Vec<u16> = origin_str.encode_utf16().collect();
        let mut result = Vec::new();
        let mut state = INIT;
        let mut i = start;
        while i < end {
            // Java 在循环内逐次调用 String#charAt；负下标或超过 UTF-16
            // 长度时抛 StringIndexOutOfBoundsException，不能静默 clamp。
            let index = usize::try_from(i).expect("java.lang.StringIndexOutOfBoundsException");
            let cur = *chars
                .get(index)
                .expect("java.lang.StringIndexOutOfBoundsException");
            i += 1;
            match state {
                INIT => {
                    if cur == b'\\' as u16 {
                        state = ESCAPE;
                    } else {
                        result.push(cur);
                    }
                }
                ESCAPE => {
                    state = INIT;
                    match cur {
                        value if value == b'b' as u16 => result.push(0x0008),
                        value if value == b't' as u16 => result.push(b'\t' as u16),
                        value if value == b'n' as u16 => result.push(b'\n' as u16),
                        value if value == b'f' as u16 => result.push(0x000C),
                        value if value == b'r' as u16 => result.push(b'\r' as u16),
                        value if value == b'"' as u16 => result.push(b'"' as u16),
                        value if value == b'\'' as u16 => result.push(b'\'' as u16),
                        value if value == b'\\' as u16 => result.push(b'\\' as u16),
                        value if value == b'$' as u16 => result.push(b'$' as u16),
                        // Java has no default branch: unknown escape vanishes.
                        _ => {}
                    }
                }
                _ => unreachable!(),
            }
        }
        JavaString::from_utf16_units(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_escapes() {
        assert_eq!(
            QLStringUtils::parse_string_escape("\"a\\nb\"").to_rust_string(),
            Some("a\nb".to_string())
        );
        assert_eq!(
            QLStringUtils::parse_string_escape("\"\\t\\r\\f\\b\"").to_rust_string(),
            Some("\t\r\u{000C}\u{0008}".to_string())
        );
        assert_eq!(
            QLStringUtils::parse_string_escape("\"\\'q\\'\"").to_rust_string(),
            Some("'q'".to_string())
        );
        assert_eq!(
            QLStringUtils::parse_string_escape("\"a\\\\b\"").to_rust_string(),
            Some("a\\b".to_string())
        );
        assert_eq!(
            QLStringUtils::parse_string_escape("\"a\\$b\"").to_rust_string(),
            Some("a$b".to_string())
        );
    }

    #[test]
    fn unknown_escape_is_dropped() {
        // No default branch in the Java switch: `\x` disappears entirely.
        assert_eq!(
            QLStringUtils::parse_string_escape("\"a\\xb\"").to_rust_string(),
            Some("ab".to_string())
        );
    }

    #[test]
    fn trailing_backslash_is_dropped() {
        // Java: state stays ESCAPE at end of loop, nothing appended.
        assert_eq!(
            QLStringUtils::parse_string_escape("\"abc\\\"").to_rust_string(),
            Some("abc".to_string())
        );
    }

    #[test]
    fn uses_java_utf16_indices_and_preserves_unpaired_surrogates() {
        let parsed = QLStringUtils::parse_string_escape("\"😀\"");
        assert_eq!(parsed.utf16_units(), &[0xD83D, 0xDE00]);

        let high = QLStringUtils::parse_string_escape_start_end("😀", 0, 1);
        assert_eq!(high.utf16_units(), &[0xD83D]);
        assert!(high.to_rust_string().is_none());

        assert!(std::panic::catch_unwind(|| {
            QLStringUtils::parse_string_escape_start_end("a", -1, 1)
        })
        .is_err());
        assert!(std::panic::catch_unwind(|| {
            QLStringUtils::parse_string_escape_start_end("a", 0, 2)
        })
        .is_err());
        assert!(QLStringUtils::parse_string_escape_start_end("a", 3, 2).is_empty());
    }
}
