//! 脚本执行的公开结果,对应 Java `com.alibaba.qlexpress4.QLResult`。
//! (VM 内部的 `QResult` 已拆至 [`crate::runtime::q_result`]。)

use crate::runtime::trace::ExpressionTrace;
use crate::runtime::value::DataValue;

/// `Express4Runner.execute` 的公开结果。对应 Java: com.alibaba.qlexpress4.QLResult
/// (携带结果值与表达式 trace 列表)
///
/// Public result of `Express4Runner.execute`, mirroring Java `QLResult`.
#[derive(Clone, Debug)]
pub struct QLResult {
    result: DataValue,
    expression_traces: Vec<ExpressionTrace>,
}

impl QLResult {
    /// 构造执行结果。对应 Java 构造器 `QLResult(result, expressionTraces)`。
    pub fn new(result: DataValue, expression_traces: Vec<ExpressionTrace>) -> Self {
        QLResult {
            result,
            expression_traces,
        }
    }

    /// 获取结果值。对应 Java 方法 `getResult`。
    pub fn result(&self) -> &DataValue {
        &self.result
    }

    /// 获取表达式 trace 列表。对应 Java 方法 `getExpressionTraces`。
    pub fn expression_traces(&self) -> &[ExpressionTrace] {
        &self.expression_traces
    }

    /// 便捷方法:消耗自身并仅返回结果值(Java 无同名方法,Rust 便捷方法)。
    /// Convenience: consume and return just the result value.
    /// 对应 Java：`QLResult#getResult()`（Rust 所有权便捷接口）。
    pub fn into_result(self) -> DataValue {
        self.result
    }
}
