//! 对应 Java 类：com.alibaba.qlexpress4.runtime.trace.ExpressionTrace
//!
//! 表达式执行追踪树的一个节点。编译期静态模型见
//! `aparser::trace_expression_visitor::TraceExpressionVisitor` 与
//! [`super::TracePointTree`]。

use super::trace_type::{java_name, TraceType};
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;

/// 记录一个表达式节点的源码范围、求值结果与嵌套子追踪。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java`；具体对象路径见 `docs/对象级对照表.md`。
/// One node of the expression-execution trace tree, mirroring Java
/// `ExpressionTrace`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.runtime.trace.ExpressionTrace。
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

    /// 返回追踪节点类型。
    /// 对应 Java: `ExpressionTrace#getTraceType`。
    pub fn trace_type(&self) -> TraceType {
        self.trace_type
    }

    /// 返回追踪节点类型。
    ///
    /// 对应 Java：`ExpressionTrace#getType()`。保留 [`Self::trace_type`] 作为
    /// Rust 描述性别名，本方法提供与 Java 公共 API 一一对应的名称。
    ///
    /// # 返回值
    ///
    /// 返回当前节点的 [`TraceType`]。
    pub fn get_type(&self) -> TraceType {
        self.trace_type
    }

    /// 返回该追踪节点对应的词法单元。
    /// 对应 Java: `ExpressionTrace#getToken`。
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 返回运行时求值得到的值；尚未求值时返回 `None`。
    /// 对应 Java: `ExpressionTrace#getValue`。
    pub fn value(&self) -> Option<&DataValue> {
        self.value.as_ref()
    }

    /// 返回当前节点是否已经记录运行时求值结果。
    /// 对应 Java: `ExpressionTrace#isEvaluated`。
    pub fn is_evaluated(&self) -> bool {
        self.evaluated
    }

    /// 返回直接子追踪节点。
    /// 对应 Java: `ExpressionTrace#getChildren`。
    pub fn children(&self) -> &[ExpressionTrace] {
        &self.children
    }

    /// 返回嵌套表达式追踪列表的可变引用。
    /// 无显式参数；返回：`&mut [ExpressionTrace]`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java`，方法 `childrenMut`；Rust 侧按所有权与 `Result` 语义适配。
    /// Mutable children access, used by instructions that mark child trace
    /// points evaluated (Java `getChildren().get(i).valueEvaluated(...)`).
    /// 对应 Java：`ExpressionTrace#getChildren()` 返回的可变 List。
    pub fn children_mut(&mut self) -> &mut [ExpressionTrace] {
        &mut self.children
    }

    /// 返回脚本中的一基行号。
    /// 对应 Java: `ExpressionTrace` 的 token 行位置。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 返回脚本中的一基列号。
    /// 对应 Java: `ExpressionTrace` 的 token 列位置。
    pub fn col(&self) -> i32 {
        self.col
    }

    /// 返回 token 在脚本中的字符偏移。
    /// 对应 Java: `ExpressionTrace` 的 token 起始位置。
    pub fn position(&self) -> i32 {
        self.position
    }

    /// 记录当前表达式已经完成求值。
    /// 参数：`value`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Value.java`，方法 `valueEvaluated`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `valueEvaluated`: record the evaluated value.
    /// 对应 Java：`ExpressionTrace#valueEvaluated(Object)`。
    pub fn value_evaluated(&mut self, value: DataValue) {
        self.value = Some(value);
        self.evaluated = true;
    }

    /// 转换为 pretty string。
    /// 参数：`indent`；返回：`String`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java`，方法 `toPrettyString`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `toPrettyString`: indented, recursive rendering.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.trace.ExpressionTrace#toPrettyString。
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
        assert_eq!(root.get_type(), TraceType::Operator);
    }
}
