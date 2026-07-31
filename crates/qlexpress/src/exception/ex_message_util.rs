//! Formats human-readable QLExpress error messages, mirroring Java
//! `ExMessageUtil`.
//!
//! Template (Java `MessageFormat`, reproduced literally):
//! `[Error {0}: {1}]\n[Near: {2}]\n{3}\n[Line: {4}, Column: {5}]`

const SNIPPET_EXTENSION_LEN: usize = 20;

pub use super::ex_message::ExMessage;

impl ExMessage {
    /// 创建格式化错误消息及其脚本片段。
    /// 承接 Java `ExMessageUtil#format` 的二元返回结果。
    /// 对应 Java: com.alibaba.qlexpress4.exception.ExMessageUtil#new。
    pub fn new(message: String, snippet: String) -> Self {
        ExMessage { message, snippet }
    }

    /// 返回完整错误消息。对应 Java: `ExMessageUtil#format` 的首个返回内容。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// 返回带定位标记的脚本片段。对应 Java: `ExMessageUtil#format` 的上下文内容。
    pub fn snippet(&self) -> &str {
        &self.snippet
    }
}

/// 根据脚本位置生成带上下文片段的错误消息。
/// 对应 Java: `com.alibaba.qlexpress4.exception.ExMessageUtil`。
pub struct ExMessageUtil;

impl ExMessageUtil {
    /// 组合错误原因、源码片段和位置生成异常文本。
    /// 参数：`script`、`token_start_pos`、`token_line`、`token_col`、`lexeme`、`error_code`、`reason`；返回：`ExMessage`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ExMessageUtil.java`，方法 `format`；Rust 侧按所有权与 `Result` 语义适配。
    /// Build the [`ExMessage`] for an error at `token_start_pos` of `script`
    /// (1-based `token_line` / `token_col`), mirroring Java
    /// `ExMessageUtil.format`.
    ///
    /// Positions use Java `String` UTF-16 code-unit offsets. Rust temporarily
    /// operates on `Vec<u16>` so snippet extension and caret width remain
    /// identical even when non-BMP characters occur before the error.
    /// 对应 Java: com.alibaba.qlexpress4.exception.ExMessageUtil#format。
    pub fn format(
        script: &str,
        token_start_pos: i32,
        token_line: i32,
        token_col: i32,
        lexeme: &str,
        error_code: &str,
        reason: &str,
    ) -> ExMessage {
        let mut units: Vec<u16> = script.encode_utf16().collect();
        let script_len = units.len() as i64;
        let lexeme_len = lexeme.encode_utf16().count() as i64;
        let token_start = i64::from(token_start_pos);
        let extension_len = SNIPPET_EXTENSION_LEN as i64;

        // 保留 Java 对任意 int 下标的循环边界行为：负 tokenStartPos 不先
        // 截断为 0，超出脚本末尾也不会切片 panic。
        let start_report_pos = (token_start - extension_len).max(0);
        let end_report_pos = (token_start + lexeme_len + extension_len).min(script_len);

        let mut snippet_builder = String::new();
        if start_report_pos > 0 {
            snippet_builder.push_str("...");
        }
        if end_report_pos > start_report_pos {
            for code_unit in &mut units[start_report_pos as usize..end_report_pos as usize] {
                // Java: chars < ' ' (control chars) are rendered as a space.
                if *code_unit < u16::from(b' ') {
                    *code_unit = u16::from(b' ');
                }
            }
            snippet_builder.push_str(&String::from_utf16_lossy(
                &units[start_report_pos as usize..end_report_pos as usize],
            ));
        }
        if end_report_pos < script_len {
            snippet_builder.push_str("...");
        }

        let mut caret_builder = String::from("       ");
        if start_report_pos > 0 {
            caret_builder.push_str("   ");
        }
        for _ in start_report_pos..token_start {
            caret_builder.push(' ');
        }
        for _ in 0..lexeme_len {
            caret_builder.push('^');
        }

        let snippet = snippet_builder;
        // 直接插值，避免连续 replace 对参数文本中的 `{2}` 等内容二次展开；
        // Java MessageFormat 只解释模板，不递归解释参数。
        let message = format!(
            "[Error {error_code}: {reason}]\n[Near: {snippet}]\n{caret_builder}\n[Line: {token_line}, Column: {token_col}]"
        );
        ExMessage::new(message, snippet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_message_like_java() {
        let script = "a = b + ";
        let ex = ExMessageUtil::format(script, 8, 1, 9, "", "SYNTAX_ERROR", "unexpected eof");
        assert_eq!(ex.snippet(), "a = b + ");
        // Caret line: 7 leading spaces + 8 spaces (chars 0..8) + 0 carets.
        let expected = "[Error SYNTAX_ERROR: unexpected eof]\n[Near: a = b + ]\n               \n[Line: 1, Column: 9]";
        assert_eq!(ex.message(), expected);
    }

    #[test]
    fn snippet_is_extended_and_control_chars_become_spaces() {
        let script = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\nb @ c dddddddddddddddddddddddddd";
        // 30 'a's at 0..29, '\n' at 30, "b @ c " from 31, '@' at index 33.
        let ex = ExMessageUtil::format(script, 33, 2, 3, "@", "SYNTAX_ERROR", "bad char");
        // start = 33-20 = 13, end = min(33+1+20, 67) = 54: chars[13..54]
        // is 17 'a's, the '\n' (renders as a space), "b @ c " and 17 'd's;
        // "..." on both ends.
        assert_eq!(
            ex.snippet(),
            "...aaaaaaaaaaaaaaaaa b @ c ddddddddddddddddd..."
        );
        // Caret line: 7 spaces + 3 ("..." prefix) + 20 (chars 13..33) + "^^^" minus 2 = 1 caret.
        let caret_line = ex.message().lines().nth(2).unwrap();
        assert_eq!(caret_line.len(), 7 + 3 + 20 + 1);
        assert!(caret_line.ends_with('^'));
        assert!(caret_line
            .chars()
            .take(caret_line.len() - 1)
            .all(|c| c == ' '));
    }

    /// SOURCE_PARITY: snippet 扩展和 caret 宽度按 Java UTF-16 单元计算。
    #[test]
    fn non_bmp_prefix_uses_utf16_caret_offset() {
        let ex = ExMessageUtil::format("😀 @", 3, 1, 4, "@", "SYNTAX_ERROR", "bad char");
        assert_eq!(ex.snippet(), "😀 @");
        let caret_line = ex.message().lines().nth(2).unwrap();
        assert_eq!(caret_line.len(), 7 + 3 + 1);
        assert!(caret_line.ends_with('^'));
    }

    /// SOURCE_PARITY: Java MessageFormat 不会递归解释参数字符串中的占位符。
    #[test]
    fn argument_placeholders_are_not_reexpanded() {
        let ex = ExMessageUtil::format("x", 0, 1, 1, "x", "E{4}", "bad {2}");
        assert!(ex.message().starts_with("[Error E{4}: bad {2}]"));
        assert_eq!(ex.snippet(), "x");
    }

    /// SOURCE_PARITY: Java 循环直接使用负 tokenStartPos，不会先截断为 0。
    #[test]
    fn negative_token_start_keeps_java_loop_boundaries() {
        let ex = ExMessageUtil::format("abcdefghijklmnopqrstuvwxyz", -5, 1, -4, "x", "E", "bad");
        assert_eq!(ex.snippet(), "abcdefghijklmnop...");
        assert_eq!(ex.message().lines().nth(2), Some("       ^"));
    }
}
