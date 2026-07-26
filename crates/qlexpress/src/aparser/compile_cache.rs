//! Compile cache value, mirroring Java `QCompileCache`.
//!
//! Java stores the compiled `QLambdaDefinition` plus expression trace
//! points; both types arrive in later stages, so the Rust port is generic
//! over the lambda definition `L` and trace point `T`. The get/put cache
//! itself is [`CompileCache`].

use std::collections::HashMap;

/// Java `QCompileCache`: the cached result of compiling one script.
#[derive(Clone, Debug)]
pub struct QCompileCache<L, T> {
    q_lambda_definition: L,
    expression_trace_points: Vec<T>,
}

impl<L, T> QCompileCache<L, T> {
    /// Java `new QCompileCache(qLambdaDefinition, expressionTracePoints)`.
    pub fn new(q_lambda_definition: L, expression_trace_points: Vec<T>) -> Self {
        QCompileCache {
            q_lambda_definition,
            expression_trace_points,
        }
    }

    /// Java `getQLambdaDefinition`.
    pub fn q_lambda_definition(&self) -> &L {
        &self.q_lambda_definition
    }

    /// Java `getExpressionTracePoints`.
    pub fn expression_trace_points(&self) -> &[T] {
        &self.expression_trace_points
    }
}

/// The concrete parse cache used by the runner (Java
/// `Express4Runner.parseCache`): script text -> compiled root
/// `QLambdaDefinition` plus expression trace points
/// (`TracePointTree`).
pub type ScriptCompileCache = CompileCache<
    std::rc::Rc<dyn crate::runtime::qlambda_definition::QLambdaDefinition>,
    crate::runtime::trace::TracePointTree,
>;

/// Script -> compiled-artifact cache with plain get/put semantics, as used
/// by the runner's parse cache (Java keeps it in `Express4Runner`).
#[derive(Clone, Debug, Default)]
pub struct CompileCache<L, T> {
    map: HashMap<String, QCompileCache<L, T>>,
}

impl<L, T> CompileCache<L, T> {
    /// 构造实例。Rust 适配接口；Java 无同名对象，承接 `CompileCache` 的同职责语义。
    pub fn new() -> Self {
        CompileCache {
            map: HashMap::new(),
        }
    }

    /// `None` when the script was never compiled (cache miss).
    pub fn get(&self, script: &str) -> Option<&QCompileCache<L, T>> {
        self.map.get(script)
    }

    /// Insert/replace the compiled artifact for `script`.
    pub fn put(&mut self, script: impl Into<String>, cache: QCompileCache<L, T>) {
        self.map.insert(script.into(), cache);
    }

    /// 执行 `len` 公开操作。Rust 适配接口；Java 无同名对象，承接 `CompileCache` 的同职责语义。
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 执行 `is_empty` 公开操作。Rust 适配接口；Java 无同名对象，承接 `CompileCache` 的同职责语义。
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 执行 `clear` 公开操作。Rust 适配接口；Java 无同名对象，承接 `CompileCache` 的同职责语义。
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
