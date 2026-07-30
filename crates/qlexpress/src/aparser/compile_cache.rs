//! Compile cache value, mirroring Java `QCompileCache`.
//!
//! Java stores the compiled `QLambdaDefinition` plus expression trace
//! points; both types arrive in later stages, so the Rust port is generic
//! over the lambda definition `L` and trace point `T`. The get/put cache
//! itself is [`CompileCache`].

use std::collections::HashMap;

/// `QCompileCache` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QCompileCache.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `QCompileCache`: the cached result of compiling one script.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.QCompileCache。
pub struct QCompileCache<L, T> {
    q_lambda_definition: L,
    expression_trace_points: Vec<T>,
}

impl<L, T> QCompileCache<L, T> {
    /// 创建对象实例。
    /// 参数：`q_lambda_definition`、`expression_trace_points`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QCompileCache.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new QCompileCache(qLambdaDefinition, expressionTracePoints)`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new(q_lambda_definition: L, expression_trace_points: Vec<T>) -> Self {
        QCompileCache {
            q_lambda_definition,
            expression_trace_points,
        }
    }

    /// 处理 q lambda definition 对应的领域职责。
    /// 无显式参数；返回：`&L`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/QLambdaDefinition.java`，方法 `qLambdaDefinition`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getQLambdaDefinition`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn q_lambda_definition(&self) -> &L {
        &self.q_lambda_definition
    }

    /// 处理 expression trace points 对应的领域职责。
    /// 无显式参数；返回：`&[T]`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/ExpressionTrace.java`，方法 `expressionTracePoints`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getExpressionTracePoints`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn expression_trace_points(&self) -> &[T] {
        &self.expression_trace_points
    }
}

/// `ScriptCompileCache` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QCompileCache.java`；具体对象路径见 `docs/对象级对照表.md`。
/// The concrete parse cache used by the runner (Java
/// `Express4Runner.parseCache`): script text -> compiled root
/// `QLambdaDefinition` plus expression trace points
/// (`TracePointTree`).
pub type ScriptCompileCache = CompileCache<
    std::rc::Rc<dyn crate::runtime::qlambda_definition::QLambdaDefinition>,
    crate::runtime::trace::TracePointTree,
>;

/// `CompileCache` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/Express4Runner.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Script -> compiled-artifact cache with plain get/put semantics, as used
/// by the runner's parse cache (Java keeps it in `Express4Runner`).
#[derive(Clone, Debug, Default)]
/// 对应 Java: 无（Rust 原生适配）。
pub struct CompileCache<L, T> {
    map: HashMap<String, QCompileCache<L, T>>,
}

impl<L, T> CompileCache<L, T> {
    /// 创建空的脚本编译缓存。
    /// 承接 Java `Express4Runner.parseCache` 的按脚本文本索引职责。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn new() -> Self {
        CompileCache {
            map: HashMap::new(),
        }
    }

    /// 处理 get 对应的领域职责。
    /// 参数：`script`；返回：`Option<&QCompileCache<L, T>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QCompileCache.java`，方法 `get`；Rust 侧按所有权与 `Result` 语义适配。
    /// `None` when the script was never compiled (cache miss).
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn get(&self, script: &str) -> Option<&QCompileCache<L, T>> {
        self.map.get(script)
    }

    /// 处理 put 对应的领域职责。
    /// 参数：`script`、`cache`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QCompileCache.java`，方法 `put`；Rust 侧按所有权与 `Result` 语义适配。
    /// Insert/replace the compiled artifact for `script`.
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn put(&mut self, script: impl Into<String>, cache: QCompileCache<L, T>) {
        self.map.insert(script.into(), cache);
    }

    /// 返回缓存中的已编译脚本数量。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 判断缓存是否没有任何编译结果。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 清空全部脚本编译结果。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_put_round_trip() {
        let mut cache: CompileCache<String, ()> = CompileCache::new();
        assert!(cache.get("1+1").is_none());
        cache.put("1+1", QCompileCache::new("lambda".to_string(), vec![]));
        assert_eq!(cache.len(), 1);
        let hit = cache.get("1+1").unwrap();
        assert_eq!(hit.q_lambda_definition(), "lambda");
        assert!(hit.expression_trace_points().is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }
}
