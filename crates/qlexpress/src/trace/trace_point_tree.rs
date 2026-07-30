//! 编译期表达式追踪点树，对应 Java
//! `com.alibaba.qlexpress4.runtime.trace.TracePointTree`。

use super::trace_type::{java_name, TraceType};
use crate::utils::println_utils::PrintlnUtils;

/// 编译期表达式追踪点。
///
/// 该对象只描述脚本中的静态位置和父子关系，不携带某次执行产生的值；
/// 运行前由 [`super::QTraces`] 转换为全新的 [`super::ExpressionTrace`] 树。
/// 对应 Java: `com.alibaba.qlexpress4.runtime.trace.TracePointTree`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TracePointTree {
    trace_type: TraceType,
    token: String,
    children: Vec<TracePointTree>,
    line: i32,
    col: i32,
    position: i32,
}

impl TracePointTree {
    /// 构造一个静态追踪点。对应 Java 构造器
    /// `TracePointTree(type, token, children, line, col, position)`。
    pub fn new(
        trace_type: TraceType,
        token: impl Into<String>,
        children: Vec<TracePointTree>,
        line: i32,
        col: i32,
        position: i32,
    ) -> Self {
        Self {
            trace_type,
            token: token.into(),
            children,
            line,
            col,
            position,
        }
    }

    /// 以 Java 相同的缩进格式递归打印。对应 Java 方法 `toPrettyString`。
    pub fn to_pretty_string(&self, indent: i32) -> String {
        let mut result = PrintlnUtils::build_indent_string(
            indent,
            &format!("{} {}", java_name(self.trace_type), self.token),
        );
        result.push('\n');
        for child in &self.children {
            result.push_str(&child.to_pretty_string(indent + 2));
        }
        result
    }

    /// 获取追踪类型。对应 Java 方法 `getType`。
    pub fn trace_type(&self) -> TraceType {
        self.trace_type
    }

    /// 获取追踪类型。
    ///
    /// 对应 Java：`TracePointTree#getType()`。保留 [`Self::trace_type`] 作为
    /// Rust 描述性别名。
    ///
    /// # 返回值
    ///
    /// 返回当前静态追踪点的 [`TraceType`]。
    pub fn get_type(&self) -> TraceType {
        self.trace_type
    }

    /// 获取源码 token 文本。对应 Java 方法 `getToken`。
    pub fn token(&self) -> &str {
        &self.token
    }

    /// 获取子追踪点。对应 Java 方法 `getChildren`。
    pub fn children(&self) -> &[TracePointTree] {
        &self.children
    }

    /// 获取源码行号。对应 Java 方法 `getLine`。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 获取源码列号（0 起始）。对应 Java 方法 `getCol`。
    pub fn col(&self) -> i32 {
        self.col
    }

    /// 获取源码绝对字符位置。对应 Java 方法 `getPosition`。
    pub fn position(&self) -> i32 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_string_matches_java_shape() {
        let point = TracePointTree::new(
            TraceType::Operator,
            "+",
            vec![TracePointTree::new(TraceType::Value, "1", vec![], 1, 0, 0)],
            1,
            1,
            1,
        );

        assert_eq!("OPERATOR +\n  | VALUE 1\n", point.to_pretty_string(0));
        assert_eq!(point.get_type(), TraceType::Operator);
    }
}
