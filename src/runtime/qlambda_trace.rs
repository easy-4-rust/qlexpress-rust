//! 带 trace 的 Lambda,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaTrace`。
//! 职责:Lambda 与其产生时捕获的 trace 集合的配对。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;

use crate::runtime::qlambda::QLambda;
use crate::runtime::trace::QTraces;

/// Lambda + 其产生时捕获的 traces。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaTrace
///
/// Lambda plus the traces captured when it was produced, mirroring Java
/// `QLambdaTrace`.
pub struct QLambdaTrace {
    q_lambda: Rc<QLambda>,
    traces: QTraces,
}

impl QLambdaTrace {
    /// 构造带 trace 的 Lambda。对应 Java 构造器 `QLambdaTrace(qLambda, traces)`。
    pub fn new(q_lambda: Rc<QLambda>, traces: QTraces) -> Self {
        QLambdaTrace { q_lambda, traces }
    }

    /// 获取 Lambda。对应 Java 方法 `getqLambda()`。
    /// Java `getqLambda()`.
    pub fn q_lambda(&self) -> &Rc<QLambda> {
        &self.q_lambda
    }

    /// 获取 trace 集合。对应 Java 方法 `getTraces()`。
    /// Java `getTraces()`.
    pub fn traces(&self) -> &QTraces {
        &self.traces
    }
}
