//! 终结符节点,对应 Java `com.alibaba.qlexpress4.aparser.TerminalNode`。
//! 职责:语法树叶子,包装一个词法 Token。
//! 本文件由 `syntax_tree.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

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
}
