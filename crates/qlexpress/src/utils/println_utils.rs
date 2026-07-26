//! Indented debug printing helpers, mirroring Java `PrintlnUtils`.

/// 构造并输出带语法树层级缩进的调试文本。
/// 对应 Java: `com.alibaba.qlexpress4.utils.PrintlnUtils`。
pub struct PrintlnUtils;

impl PrintlnUtils {
    /// Java `printlnByCurDepth`: send the indented string to `debug`.
    pub fn println_by_cur_depth(depth: i32, s: &str, debug: &mut dyn FnMut(String)) {
        debug(Self::build_indent_string(depth, s));
    }

    /// Java `buildIndentString`: indent with `"  "` per level, the last
    /// level being `"| "`.
    pub fn build_indent_string(indent: i32, origin_str: &str) -> String {
        let mut builder = String::new();
        for i in 0..indent {
            if i == indent - 1 {
                builder.push_str("| ");
            } else {
                builder.push_str("  ");
            }
        }
        builder.push_str(origin_str);
        builder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_indent_like_java() {
        assert_eq!(PrintlnUtils::build_indent_string(0, "x"), "x");
        assert_eq!(PrintlnUtils::build_indent_string(1, "x"), "| x");
        assert_eq!(PrintlnUtils::build_indent_string(3, "x"), "    | x");
    }

    #[test]
    fn println_sends_to_consumer() {
        let mut out = Vec::new();
        PrintlnUtils::println_by_cur_depth(2, "tok", &mut |s| out.push(s));
        assert_eq!(out, vec!["  | tok".to_string()]);
    }
}
