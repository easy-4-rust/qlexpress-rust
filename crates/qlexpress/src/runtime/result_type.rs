//! QVM 控制流结果类别。
//!
//! 来源对象：`com.alibaba.qlexpress4.runtime.QResult.ResultType`。

/// QVM 控制流结果类别。
///
/// 对应 Java：`QResult.ResultType`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultType {
    /// 跳出循环。
    Break,
    /// 继续下一次循环。
    Continue,
    /// 跳转到指定指令。
    Jump,
    /// 从函数、Lambda 或脚本返回。
    Return,
    /// 顺序执行下一条指令。
    NextInstruction,
}
