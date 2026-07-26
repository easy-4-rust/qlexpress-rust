//! 对应 Java 类：com.alibaba.qlexpress4.runtime.trace.ExpressionTrace
//!
//! 表达式执行追踪树的一个节点。编译期静态模型见
//! `aparser::trace_expression_visitor::TraceExpressionVisitor` 与
//! [`super::TracePointTree`]。

use super::trace_type::{java_name, TraceType};
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;

/// One node of the expression-execution trace tree, mirroring Java
/// `ExpressionTrace`.
#[derive(Clone, Debug)]
pub struct ExpressionTrace {
    trace_type: TraceType,
    token: String,
    /// Intermediate calculation result of this trace point.
    value: Option<DataValue>,
    /// True if this point was evaluated in this execution
    /// (false when short-circuited).
    evaluated: bool,
    children: Vec<ExpressionTrace>,
    /// 1-based line number in the source.
    line: i32,
    /// 1-based column number in the source.
    col: i32,
    /// Absolute character position in the source string.
    position: i32,
}

impl ExpressionTrace {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:9` 的 `ExpressionTrace::<init>`。
    pub fn new(
        trace_type: TraceType,
        token: impl Into<String>,
        children: Vec<ExpressionTrace>,
        line: i32,
        col: i32,
        position: i32,
    ) -> Self {
        ExpressionTrace {
            trace_type,
            token: token.into(),
            value: None,
            evaluated: false,
            children,
            line,
            col,
            position,
        }
    }

    /// 执行 `trace_type` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn trace_type(&self) -> TraceType {
        self.trace_type
    }

    /// 执行 `token` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 执行 `value` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn value(&self) -> Option<&DataValue> {
        self.value.as_ref()
    }

    /// 执行 `is_evaluated` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:77` 的 `ExpressionTrace#isEvaluated`。
    pub fn is_evaluated(&self) -> bool {
        self.evaluated
    }

    /// 执行 `children` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn children(&self) -> &[ExpressionTrace] {
        &self.children
    }

    /// Mutable children access, used by instructions that mark child trace
    /// points evaluated (Java `getChildren().get(i).valueEvaluated(...)`).
    pub fn children_mut(&mut self) -> &mut [ExpressionTrace] {
        &mut self.children
    }

    /// 执行 `line` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 执行 `col` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn col(&self) -> i32 {
        self.col
    }

    /// 执行 `position` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java:1` 的 `ExpressionTrace`；该方法为 Rust 同职责适配接口。
    pub fn position(&self) -> i32 {
        self.position
    }

    /// Java `valueEvaluated`: record the evaluated value.
    pub fn value_evaluated(&mut self, value: DataValue) {
        self.value = Some(value);
        self.evaluated = true;
    }

    /// Java `toPrettyString`: indented, recursive rendering.
    pub fn to_pretty_string(&self, indent: i32) -> String {
        let value_part = if self.evaluated {
            match &self.value {
                Some(v) => v.string_value_of(),
                None => String::new(),
            }
        } else {
            String::new()
        };
        let mut result = PrintlnUtils::build_indent_string(
            indent,
            &format!(
                "{} {} {}",
                java_name(self.trace_type),
                self.token,
                value_part
            ),
        );
        result.push('\n');
        for child in &self.children {
            result.push_str(&child.to_pretty_string(indent + 2));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_prints_tree() {
        let mut leaf = ExpressionTrace::new(TraceType::Value, "1", vec![], 1, 5, 4);
        leaf.value_evaluated(DataValue::Int(1));
        let root = ExpressionTrace::new(TraceType::Operator, "+", vec![leaf], 1, 3, 2);
        let pretty = root.to_pretty_string(0);
        assert!(pretty.starts_with("OPERATOR + \n"));
        assert!(pretty.contains("| VALUE 1 1"));
    }
}
