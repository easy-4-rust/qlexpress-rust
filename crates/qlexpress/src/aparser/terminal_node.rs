//! 终结符节点,对应 Java `com.alibaba.qlexpress4.aparser.TerminalNode`。
//! 职责:语法树叶子,包装一个词法 Token。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use super::qlparser_base_visitor::Visitor;
use super::token::Token;

/// 终结符节点:语法树的叶子,包装一个词法 [`Token`]。
/// 对应 Java: com.alibaba.qlexpress4.aparser.TerminalNode(语法树叶子节点)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalNode {
    symbol: Token,
}

impl TerminalNode {
    /// 构造终结符节点,对应 Java 构造器 `TerminalNode(Token)`。
    pub fn new(symbol: Token) -> Self {
        TerminalNode { symbol }
    }

    /// 获取包装的 Token。对应 Java 方法 `getSymbol`。
    /// Java `getSymbol`.
    pub fn symbol(&self) -> &Token {
        &self.symbol
    }

    /// 获取 token 文本。对应 Java 方法 `getText`。
    /// Java `getText` (the token text).
    pub fn text(&self) -> &str {
        self.symbol.text()
    }

    /// 双分派到 Visitor 的终结符处理方法。
    ///
    /// 对应 Java：`TerminalNode#accept(QLParserBaseVisitor)`。
    ///
    /// # 参数
    /// - `visitor`：接收当前终结符的语法树访问器。
    ///
    /// # 返回值
    /// 返回访问器的终结符访问结果。
    pub fn accept<V: Visitor + ?Sized>(&self, visitor: &mut V) -> V::T {
        visitor.visit_terminal(self)
    }
}

impl std::fmt::Display for TerminalNode {
    /// 按 Java `TerminalNode#toString()` 规则输出 token 文本。
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aparser::token::ID;

    struct TerminalTextVisitor;

    impl Visitor for TerminalTextVisitor {
        type T = String;

        fn visit_terminal(&mut self, node: &TerminalNode) -> Self::T {
            node.text().to_string()
        }
    }

    /// `SOURCE_PARITY`：Java `accept` 分派到 `visitTerminal`，`toString`
    /// 返回与 `getText` 相同的 token 文本。
    #[test]
    fn accept_and_display_match_java_terminal_contract() {
        let terminal = TerminalNode::new(Token::new(ID as i32, "name", 0, 3, 1, 0));
        assert_eq!(terminal.accept(&mut TerminalTextVisitor), "name");
        assert_eq!(terminal.to_string(), "name");
    }
}
