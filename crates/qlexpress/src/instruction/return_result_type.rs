//! 返回、跳出循环与继续循环的结果类别。

/// `ReturnInstruction` 写入 QVM 的控制流结果类型。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.QResult.ResultType`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnResultType {
    /// 函数或脚本返回。
    Return,
    /// 跳出当前循环。
    Break,
    /// 继续下一次循环。
    Continue,
}
