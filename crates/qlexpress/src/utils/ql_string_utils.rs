//! String escape parsing, mirroring Java `QLStringUtils`.

/// 按 QLExpress 字符串字面量规则解析引号和转义序列。
/// 对应 Java: `com.alibaba.qlexpress4.utils.QLStringUtils`。
pub struct QLStringUtils;

impl QLStringUtils {
    /// 构建或解析 string escape。
    /// 参数：`origin_str`；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/QLStringUtils.java`，方法 `parseStringEscape`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `parseStringEscape`: strip the first and last character (the
    /// surrounding quotes) and resolve escape sequences.
    /// 对应 Java：`QLStringUtils#parseStringEscape(String)`。
    pub fn parse_string_escape(origin_str: &str) -> String {
        let chars: Vec<char> = origin_str.chars().collect();
        let end = chars.len().saturating_sub(1);
        Self::parse_string_escape_start_end(origin_str, 1, end)
    }

    /// 构建或解析 string escape start end。
    /// 参数：`origin_str`、`start`、`end`；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/utils/QLStringUtils.java`，方法 `parseStringEscapeStartEnd`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `parseStringEscapeStartEnd`: resolve escapes in
    /// `origin_str[start..end]` (char indices).
    ///
    /// Recognized escapes: `\b \t \n \f \r \" \' \\ \$`. An unknown escape
    /// silently drops both characters (as in the Java switch with no
    /// default branch).
    /// 对应 Java: com.alibaba.qlexpress4.utils.QLStringUtils#parseStringEscapeStartEnd。
    pub fn parse_string_escape_start_end(origin_str: &str, start: usize, end: usize) -> String {
        const INIT: u8 = 0;
        const ESCAPE: u8 = 1;

        let chars: Vec<char> = origin_str.chars().collect();
        let end = end.min(chars.len());
        let mut result = String::new();
        let mut state = INIT;
        let mut i = start.min(end);
        while i < end {
            let cur = chars[i];
            i += 1;
            match state {
                INIT => {
                    if cur == '\\' {
                        state = ESCAPE;
                    } else {
                        result.push(cur);
                    }
                }
                ESCAPE => {
                    state = INIT;
                    match cur {
                        'b' => result.push('\u{0008}'),
                        't' => result.push('\t'),
                        'n' => result.push('\n'),
                        'f' => result.push('\u{000C}'),
                        'r' => result.push('\r'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        '\\' => result.push('\\'),
                        '$' => result.push('$'),
                        // Java has no default branch: unknown escape vanishes.
                        _ => {}
                    }
                }
                _ => unreachable!(),
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_escapes() {
        assert_eq!(QLStringUtils::parse_string_escape("\"a\\nb\""), "a\nb");
        assert_eq!(
            QLStringUtils::parse_string_escape("\"\\t\\r\\f\\b\""),
            "\t\r\u{000C}\u{0008}"
        );
        assert_eq!(QLStringUtils::parse_string_escape("\"\\'q\\'\""), "'q'");
        assert_eq!(QLStringUtils::parse_string_escape("\"a\\\\b\""), "a\\b");
        assert_eq!(QLStringUtils::parse_string_escape("\"a\\$b\""), "a$b");
    }

    #[test]
    fn unknown_escape_is_dropped() {
        // No default branch in the Java switch: `\x` disappears entirely.
        assert_eq!(QLStringUtils::parse_string_escape("\"a\\xb\""), "ab");
    }

    #[test]
    fn trailing_backslash_is_dropped() {
        // Java: state stays ESCAPE at end of loop, nothing appended.
        assert_eq!(QLStringUtils::parse_string_escape("\"abc\\\""), "abc");
    }
}
