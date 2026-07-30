//! QVM 异常表中的单个处理器条目。

/// 将一个程序计数器区间和异常类型映射到处理器地址。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.ExceptionTable.ExceptionTableEntry`。
#[derive(Clone, Debug)]
pub struct ExceptionTableEntry {
    /// 异常处理区间的起始指令位置（包含）。
    pub start_pc: usize,
    /// 异常处理区间的结束指令位置（不包含）。
    pub end_pc: usize,
    /// 匹配异常后跳转的处理器指令位置。
    pub handler_pc: usize,
    /// 可捕获的 Java 异常类型；为空表示 finally/catch-all。
    pub catch_type: Option<String>,
}
