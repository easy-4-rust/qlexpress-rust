//! 运行时表达式追踪注册表，对应 Java
//! `com.alibaba.qlexpress4.runtime.trace.QTraces`。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::trace::{ExpressionTrace, TracePointTree};

/// 一个追踪点在根树中的路径：第一个元素是根下标，其余元素是逐级子节点下标。
type TracePath = Vec<usize>;

/// 保存本次执行的表达式追踪树及 `traceKey -> 节点路径` 索引。
///
/// Java 以对象引用同时保存根树和位置索引；Rust 的子节点是拥有所有权的
/// `Vec<ExpressionTrace>`，因此以稳定路径定位同一节点，避免复制节点后只更新
/// 索引副本。对应 Java: `com.alibaba.qlexpress4.runtime.trace.QTraces`。
#[derive(Clone, Debug, Default)]
pub struct QTraces {
    expression_traces: Rc<RefCell<Vec<ExpressionTrace>>>,
    expression_trace_map: Rc<HashMap<i32, TracePath>>,
}

impl QTraces {
    /// 由已构造的运行时树和路径索引创建注册表。
    ///
    /// 对应 Java 构造器 `QTraces(expressionTraces, expressionTraceMap)`；
    /// 此入口主要用于底层测试，生产执行通常使用 [`Self::from_trace_points`]。
    pub fn new(
        expression_traces: Vec<ExpressionTrace>,
        expression_trace_map: HashMap<i32, TracePath>,
    ) -> Self {
        QTraces {
            expression_traces: Rc::new(RefCell::new(expression_traces)),
            expression_trace_map: Rc::new(expression_trace_map),
        }
    }

    /// 从编译期追踪点创建一次执行专属的运行时树。
    ///
    /// 对应 Java `Express4Runner.convertPoints2QTraces`：每次执行都新建
    /// `ExpressionTrace`，并以源码绝对位置作为指令使用的 trace key。
    pub fn from_trace_points(trace_points: &[TracePointTree]) -> Self {
        let mut expression_trace_map = HashMap::new();
        let expression_traces = trace_points
            .iter()
            .enumerate()
            .map(|(root_index, point)| {
                let mut path = vec![root_index];
                build_expression_trace(point, &mut path, &mut expression_trace_map)
            })
            .collect();
        Self::new(expression_traces, expression_trace_map)
    }

    /// 创建空注册表。对应 Java 中 `expressionTraceMap == null` 的情形。
    pub fn empty() -> Self {
        QTraces::default()
    }

    /// 按 trace key 获取节点快照。对应 Java 方法 `getExpressionTraceByKey`。
    ///
    /// Rust 返回拥有所有权的快照；指令修改节点应使用
    /// [`Self::with_expression_trace_by_key`]，以保证修改的是根树中的真实节点。
    pub fn get_expression_trace_by_key(&self, trace_key: Option<i32>) -> Option<ExpressionTrace> {
        let path = trace_key.and_then(|key| self.expression_trace_map.get(&key))?;
        trace_at_path(&self.expression_traces.borrow(), path).cloned()
    }

    /// 对命中的运行时追踪节点执行原地修改。
    ///
    /// 这是 Java 取到对象引用后调用 `valueEvaluated` 的 Rust 等价实现。
    pub fn with_expression_trace_by_key(
        &self,
        trace_key: Option<i32>,
        f: impl FnOnce(&mut ExpressionTrace),
    ) {
        let Some(path) = trace_key.and_then(|key| self.expression_trace_map.get(&key)) else {
            return;
        };
        if let Some(trace) = trace_at_path_mut(&mut self.expression_traces.borrow_mut(), path) {
            f(trace);
        }
    }

    /// 返回运行时追踪根树快照。对应 Java 方法 `getExpressionTraces`。
    pub fn snapshot(&self) -> Vec<ExpressionTrace> {
        self.expression_traces.borrow().clone()
    }
}

fn build_expression_trace(
    point: &TracePointTree,
    path: &mut TracePath,
    expression_trace_map: &mut HashMap<i32, TracePath>,
) -> ExpressionTrace {
    expression_trace_map.insert(point.position(), path.clone());
    let children = point
        .children()
        .iter()
        .enumerate()
        .map(|(child_index, child)| {
            path.push(child_index);
            let trace = build_expression_trace(child, path, expression_trace_map);
            path.pop();
            trace
        })
        .collect();
    ExpressionTrace::new(
        point.trace_type(),
        point.token(),
        children,
        point.line(),
        point.col(),
        point.position(),
    )
}

fn trace_at_path<'a>(
    expression_traces: &'a [ExpressionTrace],
    path: &[usize],
) -> Option<&'a ExpressionTrace> {
    let (root_index, child_path) = path.split_first()?;
    let mut current = expression_traces.get(*root_index)?;
    for child_index in child_path {
        current = current.children().get(*child_index)?;
    }
    Some(current)
}

fn trace_at_path_mut<'a>(
    expression_traces: &'a mut [ExpressionTrace],
    path: &[usize],
) -> Option<&'a mut ExpressionTrace> {
    let (root_index, child_path) = path.split_first()?;
    let mut current = expression_traces.get_mut(*root_index)?;
    for child_index in child_path {
        current = current.children_mut().get_mut(*child_index)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::trace::TraceType;
    use crate::runtime::value::DataValue;

    #[test]
    fn updates_nested_node_in_root_tree() {
        let traces = QTraces::from_trace_points(&[TracePointTree::new(
            TraceType::Operator,
            "+",
            vec![TracePointTree::new(TraceType::Value, "1", vec![], 1, 0, 0)],
            1,
            1,
            1,
        )]);

        traces.with_expression_trace_by_key(Some(0), |trace| {
            trace.value_evaluated(DataValue::Int(1));
        });

        let snapshot = traces.snapshot();
        assert!(snapshot[0].children()[0].is_evaluated());
        assert_eq!(Some(&DataValue::Int(1)), snapshot[0].children()[0].value());
    }

    #[test]
    fn each_execution_gets_fresh_trace_values() {
        let points = [TracePointTree::new(TraceType::Value, "1", vec![], 1, 0, 0)];
        let first = QTraces::from_trace_points(&points);
        first.with_expression_trace_by_key(Some(0), |trace| {
            trace.value_evaluated(DataValue::Int(1));
        });
        let second = QTraces::from_trace_points(&points);

        assert!(first.snapshot()[0].is_evaluated());
        assert!(!second.snapshot()[0].is_evaluated());
    }
}
