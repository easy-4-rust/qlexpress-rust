//! 二元操作符 trait 定义。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.BinaryOperator
//! (interface,extends Operator;`CustomBinaryOperator` 为用户自定义
//! 操作符的入口,两者由 OperatorInstruction 同样分派)。
//!
//! 说明:本 trait 原定义于 base 模块,Stage 4a 按 SPEC §5.5 一类一文件
//! 迁至本文件(Java BinaryOperator.java 位于 operator 顶层包);
//! `base` 模块 re-export 之,既有引用 `base::BinaryOperator` 不受影响。

use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// 二元操作符。
///
/// 对应 Java: BinaryOperator(interface BinaryOperator extends Operator)。
pub trait BinaryOperator {
    /// 执行操作符计算。
    ///
    /// 对应 Java 方法: `execute(Value left, Value right, QRuntime qRuntime,
    /// QLOptions qlOptions, ErrorReporter errorReporter)`,返回 Java `Object`。
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException>;

    /// 对应 Java 方法: `Operator.getOperator()` —— 操作符词素(如 `"+"`)。
    fn operator(&self) -> &str;

    /// 对应 Java 方法: `Operator.getPriority()` —— 优先级(见 QLPrecedences)。
    fn priority(&self) -> i32;
}
