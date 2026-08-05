//! Operator metadata consulted by the lexer/parser, mirroring the Java
//! `ParserOperatorManager` interface.
//!
//! Stage 1 only needs the contract (the lexer calls
//! [`ParserOperatorManager::get_alias`] when scanning identifiers); the
//! concrete implementation backed by the runtime operator table is Stage-2+
//! work.

pub use super::op_type::OpType;

/// `ParserOperatorManager` 接口的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ParserOperatorManager.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `ParserOperatorManager`.
///
/// determine whether a lexeme is an operator of a given type, query binary
/// operator precedence, and map word operators (e.g. `and`) to their aliased
/// token type.
/// 对应 Java: com.alibaba.qlexpress4.aparser.ParserOperatorManager。
pub trait ParserOperatorManager {
    /// 判断 op type 条件。
    /// 参数：`lexeme`、`op_type`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ParserOperatorManager.java`，方法 `isOpType`。
    /// Determine whether `lexeme` is an operator of `op_type`
    /// (Java `isOpType`).
    fn is_op_type(&self, lexeme: &str, op_type: OpType) -> bool;

    /// 处理 precedence 对应的接口职责。
    /// 参数：`lexeme`；返回：`Option<i32>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ParserOperatorManager.java`，方法 `precedence`。
    /// Binary operator precedence of `lexeme`;接口允许非操作符返回 `None`。
    /// Java 基线的 `OperatorManager` 实现实际会在该输入上抛 NPE，因此
    /// Rust 的对应实现也保留该行为；其他 trait 实现仍可返回 `None`。
    fn precedence(&self, lexeme: &str) -> Option<i32>;

    /// 查询 alias。
    /// 参数：`lexeme`；返回：`Option<i32>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/ParserOperatorManager.java`，方法 `getAlias`。
    /// Aliased token type of `lexeme`; `None` if there is no alias
    /// (Java `getAlias`).
    fn get_alias(&self, lexeme: &str) -> Option<i32>;
}
