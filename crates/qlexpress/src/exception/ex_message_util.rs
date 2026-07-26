//! Formats human-readable QLExpress error messages, mirroring Java
//! `ExMessageUtil`.
//!
//! Template (Java `MessageFormat`, reproduced literally):
//! `[Error {0}: {1}]\n[Near: {2}]\n{3}\n[Line: {4}, Column: {5}]`

const REPORT_TEMPLATE: &str = "[Error {0}: {1}]\n[Near: {2}]\n{3}\n[Line: {4}, Column: {5}]";

const SNIPPET_EXTENSION_LEN: usize = 20;

/// `ExMessage` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ExMessageUtil.java`；具体对象路径见 `docs/对象级对照表.md`。
/// A formatted error message plus the snippet extracted around the error,
/// mirroring Java `ExMessageUtil.ExMessage`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExMessage {
    message: String,
    snippet: String,
}

impl ExMessage {
    /// 创建格式化错误消息及其脚本片段。
    /// 承接 Java `ExMessageUtil#format` 的二元返回结果。
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
    /// 处理 format 对应的领域职责。
    /// 参数：`script`、`token_start_pos`、`token_line`、`token_col`、`lexeme`、`error_code`、`reason`；返回：`ExMessage`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/ExMessageUtil.java`，方法 `format`；Rust 侧按所有权与 `Result` 语义适配。
    /// Build the [`ExMessage`] for an error at `token_start_pos` of `script`
    /// (1-based `token_line` / `token_col`), mirroring Java
    /// `ExMessageUtil.format`.
    ///
    /// Positions are in characters (Java `String.charAt` semantics); this
    /// port operates on `Vec<char>` to stay correct for non-ASCII scripts.
    pub fn format(
        script: &str,
        token_start_pos: i32,
        token_line: i32,
        token_col: i32,
        lexeme: &str,
        error_code: &str,
        reason: &str,
    ) -> ExMessage {
        let chars: Vec<char> = script.chars().collect();
        let lexeme_len = lexeme.chars().count();
        let token_start = token_start_pos.max(0) as usize;

        let start_report_pos = token_start.saturating_sub(SNIPPET_EXTENSION_LEN);
        let end_report_pos = (token_start + lexeme_len + SNIPPET_EXTENSION_LEN).min(chars.len());

        let mut snippet_builder = String::new();
        if start_report_pos > 0 {
            snippet_builder.push_str("...");
        }
        for &code_char in &chars[start_report_pos..end_report_pos] {
            // Java: chars < ' ' (control chars) are rendered as a space.
            snippet_builder.push(if code_char < ' ' { ' ' } else { code_char });
        }
        if end_report_pos < chars.len() {
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
}
