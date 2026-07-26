//! Operator metadata consulted by the lexer/parser, mirroring the Java
//! `ParserOperatorManager` interface.
//!
//! Stage 1 only needs the contract (the lexer calls
//! [`ParserOperatorManager::get_alias`] when scanning identifiers); the
//! concrete implementation backed by the runtime operator table is Stage-2+
//! work.

/// Java `ParserOperatorManager.OpType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OpType {
    /// Prefix (unary) operator, e.g. `!x`.
    Prefix,
    /// Suffix (postfix) operator, e.g. `x++`.
    Suffix,
    /// Infix (binary) operator, e.g. `a + b`.
    Middle,
}

/// Java `ParserOperatorManager`.
///
/// determine whether a lexeme is an operator of a given type, query binary
/// operator precedence, and map word operators (e.g. `and`) to their aliased
/// token type.
pub trait ParserOperatorManager {
    /// Determine whether `lexeme` is an operator of `op_type`
    /// (Java `isOpType`).
    fn is_op_type(&self, lexeme: &str, op_type: OpType) -> bool;

    /// Binary operator precedence of `lexeme`; `None` if it is not an
    /// operator (Java `precedence`, which returns `null` in that case).
    fn precedence(&self, lexeme: &str) -> Option<i32>;

    /// Aliased token type of `lexeme`; `None` if there is no alias
    /// (Java `getAlias`).
    fn get_alias(&self, lexeme: &str) -> Option<i32>;
}
