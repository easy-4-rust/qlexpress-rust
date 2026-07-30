//! 递归下降解析器的回溯与致命错误分支。

use crate::exception::QLSyntaxException;

/// 区分可回溯的试探失败与用户可见语法错误。
///
/// 对应 Java: `QLParser.QLParseBacktrack` 与
/// `QLSyntaxException` 两条控制流。
#[derive(Debug)]
pub enum ParseFail {
    /// 试探解析未命中，调用者应恢复位置并选择其他产生式。
    Backtrack,
    /// 不可回溯的用户可见语法错误。
    Syntax(QLSyntaxException),
}
