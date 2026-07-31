//! 递归下降解析器的回溯与致命错误分支。

use crate::exception::QLSyntaxException;

/// 区分可回溯的试探失败与用户可见语法错误。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.QLParser.QLParseBacktrack` 与
/// `QLSyntaxException` 两条控制流。
#[derive(Debug)]
// 语法异常按值传播是公开错误模型的一部分；为只含两个分支的内部控制流
// 引入堆分配会改变所有解析热路径，故明确接受尺寸差异。
#[allow(clippy::large_enum_variant)]
pub enum ParseFail {
    /// 试探解析未命中，调用者应恢复位置并选择其他产生式。
    Backtrack,
    /// 不可回溯的用户可见语法错误。
    Syntax(QLSyntaxException),
}
