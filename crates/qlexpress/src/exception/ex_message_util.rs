//! Formats human-readable QLExpress error messages, mirroring Java
//! `ExMessageUtil`.
//!
//! Template (Java `MessageFormat`, reproduced literally):
//! `[Error {0}: {1}]\n[Near: {2}]\n{3}\n[Line: {4}, Column: {5}]`

const REPORT_TEMPLATE: &str = "[Error {0}: {1}]\n[Near: {2}]\n{3}\n[Line: {4}, Column: {5}]";

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
        let lexeme_len = lexeme.encode_utf16().count();
        let token_start = token_start_pos.max(0) as usize;

        let start_report_pos = token_start.saturating_sub(SNIPPET_EXTENSION_LEN);
        let end_report_pos =
            (token_start + lexeme_len + SNIPPET_EXTENSION_LEN).min(units.len());

        let mut snippet_builder = String::new();
        if start_report_pos > 0 {
            snippet_builder.push_str("...");
        }
        for code_unit in &mut units[start_report_pos..end_report_pos] {
            // Java: chars < ' ' (control chars) are rendered as a space.
            if *code_unit < u16::from(b' ') {
                *code_unit = u16::from(b' ');
            }
        }
        snippet_builder.push_str(&String::from_utf16_lossy(
            &units[start_report_pos..end_report_pos],
        ));
        if end_report_pos < units.len() {
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
        let message = REPORT_TEMPLATE
            .replace("{0}", error_code)
            .replace("{1}", reason)
            .replace("{2}", &snippet)
            .replace("{3}", &caret_builder)
            .replace("{4}", &token_line.to_string())
            .replace("{5}", &token_col.to_string());
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
}
