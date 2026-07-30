//! Execution options, mirroring Java `QLOptions` (Builder pattern).

use std::collections::HashMap;

use crate::runtime::value::DataValue;

/// `Attachments` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/QLOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Attachments carried to user-defined function/operator/macro; Java uses
/// `Map<String, Object>`, Rust uses script values.
pub type Attachments = HashMap<String, DataValue>;

/// 单次脚本执行的 Java 兼容选项集合。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/QLOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Per-execution options, mirroring Java `QLOptions`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.QLOptions。
pub struct QLOptions {
    /// Precise evaluate based on BigDecimal. Default false.
    precise: bool,
    /// Define global symbols in the user context. Default false.
    pollute_user_context: bool,
    /// Script timeout in milliseconds; `<= 0` means unlimited. Default -1.
    timeout_millis: i64,
    /// Attachments passed to user-defined functions/operators/macros; only
    /// used to pass data, never as variable values. Default empty.
    attachments: Attachments,
    /// Allow caching the compile result of the script. Default false.
    cache: bool,
    /// Avoid null pointer. Default false.
    avoid_null_pointer: bool,
    /// Max length of arrays allowed to be created; -1 means no limit.
    /// Default -1.
    max_arr_length: i32,
    /// Track execution of all expressions and return the path to the
    /// `execute` caller. Requires `InitOptions::trace_expression` too.
    /// Default false.
    trace_expression: bool,
    /// Disable short circuit in logic operators. Default false.
    short_circuit_disable: bool,
}

impl QLOptions {
    /// 创建单次执行选项构建器。对应 Java: `QLOptions#builder`。
    pub fn builder() -> QLOptionsBuilder {
        QLOptionsBuilder::new()
    }

    /// 返回是否启用精确数值计算。对应 Java: `QLOptions#isPrecise`。
    pub fn is_precise(&self) -> bool {
        self.precise
    }

    /// 返回脚本定义是否写回用户上下文。对应 Java: `QLOptions#isPolluteUserContext`。
    pub fn is_pollute_user_context(&self) -> bool {
        self.pollute_user_context
    }

    /// 返回执行超时毫秒数；非正值表示无限制。对应 Java: `QLOptions#timeoutMillis`。
    pub fn timeout_millis(&self) -> i64 {
        self.timeout_millis
    }

    /// 返回传递给自定义扩展的只读附件。对应 Java: `QLOptions#attachments`。
    pub fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    /// 返回是否缓存脚本编译结果。对应 Java: `QLOptions#isCache`。
    pub fn is_cache(&self) -> bool {
        self.cache
    }

    /// 返回是否启用空指针规避。对应 Java: `QLOptions#isAvoidNullPointer`。
    pub fn is_avoid_null_pointer(&self) -> bool {
        self.avoid_null_pointer
    }

    /// 返回脚本可创建的最大数组长度；`-1` 表示无限制。对应 Java: `QLOptions#maxArrLength`。
    pub fn max_arr_length(&self) -> i32 {
        self.max_arr_length
    }

    /// 校验 arr len。
    /// 参数：`new_arr_len`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/QLOptions.java`，方法 `checkArrLen`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `checkArrLen`: true when `new_arr_len` is within the limit
    /// (unlimited when `max_arr_length == -1`).
    pub fn check_arr_len(&self, new_arr_len: i32) -> bool {
        self.max_arr_length == -1 || new_arr_len <= self.max_arr_length
    }

    /// 返回本次执行是否采集表达式追踪。对应 Java: `QLOptions#isTraceExpression`。
    pub fn is_trace_expression(&self) -> bool {
        self.trace_expression
    }

    /// 返回是否禁用逻辑短路。对应 Java: `QLOptions#isShortCircuitDisable`。
    pub fn is_short_circuit_disable(&self) -> bool {
        self.short_circuit_disable
    }
}

impl Default for QLOptions {
    /// Java `QLOptions.DEFAULT_OPTIONS`.
    fn default() -> Self {
        QLOptions::builder().build()
    }
}

/// 以链式 API 构造 [`QLOptions`] 的构建器。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/QLOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `QLOptions.Builder`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.QLOptions。
pub struct QLOptionsBuilder {
    precise: bool,
    pollute_user_context: bool,
    timeout_millis: i64,
    attachments: Attachments,
    cache: bool,
    avoid_null_pointer: bool,
    max_arr_length: i32,
    trace_expression: bool,
    short_circuit_disable: bool,
}

impl QLOptionsBuilder {
    /// 创建采用 Java 默认执行参数的构建器。对应 Java: `QLOptions.Builder`。
    pub fn new() -> Self {
        QLOptionsBuilder {
            precise: false,
            pollute_user_context: false,
            timeout_millis: -1,
            attachments: HashMap::new(),
            cache: false,
            avoid_null_pointer: false,
            max_arr_length: -1,
            trace_expression: false,
            short_circuit_disable: false,
        }
    }

    /// 设置精确计算开关并返回构建器。对应 Java: `QLOptions.Builder#precise`。
    pub fn precise(mut self, precise: bool) -> Self {
        self.precise = precise;
        self
    }

    /// 设置是否写回用户上下文并返回构建器。对应 Java: `QLOptions.Builder#polluteUserContext`。
    pub fn pollute_user_context(mut self, pollute_user_context: bool) -> Self {
        self.pollute_user_context = pollute_user_context;
        self
    }

    /// 设置执行超时毫秒数并返回构建器。对应 Java: `QLOptions.Builder#timeoutMillis`。
    pub fn timeout_millis(mut self, timeout_millis: i64) -> Self {
        self.timeout_millis = timeout_millis;
        self
    }

    /// 设置传递给自定义扩展的附件并返回构建器。对应 Java: `QLOptions.Builder#attachments`。
    pub fn attachments(mut self, attachments: Attachments) -> Self {
        self.attachments = attachments;
        self
    }

    /// 设置编译缓存开关并返回构建器。对应 Java: `QLOptions.Builder#cache`。
    pub fn cache(mut self, cache: bool) -> Self {
        self.cache = cache;
        self
    }

    /// 设置空指针规避开关并返回构建器。对应 Java: `QLOptions.Builder#avoidNullPointer`。
    pub fn avoid_null_pointer(mut self, avoid_null_pointer: bool) -> Self {
        self.avoid_null_pointer = avoid_null_pointer;
        self
    }

    /// 设置最大数组长度并返回构建器。对应 Java: `QLOptions.Builder#maxArrLength`。
    pub fn max_arr_length(mut self, max_arr_length: i32) -> Self {
        self.max_arr_length = max_arr_length;
        self
    }

    /// 设置表达式追踪开关并返回构建器。对应 Java: `QLOptions.Builder#traceExpression`。
    pub fn trace_expression(mut self, trace_expression: bool) -> Self {
        self.trace_expression = trace_expression;
        self
    }

    /// 设置逻辑短路禁用开关并返回构建器。对应 Java: `QLOptions.Builder#shortCircuitDisable`。
    pub fn short_circuit_disable(mut self, short_circuit_disable: bool) -> Self {
        self.short_circuit_disable = short_circuit_disable;
        self
    }

    /// 构建不可变执行选项。对应 Java: `QLOptions.Builder#build`。
    pub fn build(self) -> QLOptions {
        QLOptions {
            precise: self.precise,
            pollute_user_context: self.pollute_user_context,
            timeout_millis: self.timeout_millis,
            attachments: self.attachments,
            cache: self.cache,
            avoid_null_pointer: self.avoid_null_pointer,
            max_arr_length: self.max_arr_length,
            trace_expression: self.trace_expression,
            short_circuit_disable: self.short_circuit_disable,
        }
    }
}

impl Default for QLOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_java() {
        let opts = QLOptions::default();
        assert!(!opts.is_precise());
        assert!(!opts.is_pollute_user_context());
        assert_eq!(opts.timeout_millis(), -1);
        assert!(opts.attachments().is_empty());
        assert!(!opts.is_cache());
        assert!(!opts.is_avoid_null_pointer());
        assert_eq!(opts.max_arr_length(), -1);
        assert!(!opts.is_trace_expression());
        assert!(!opts.is_short_circuit_disable());
    }

    #[test]
    fn builder_sets_all_fields() {
        let mut attachments = HashMap::new();
        attachments.insert("k".to_string(), DataValue::Int(1));
        let opts = QLOptions::builder()
            .precise(true)
            .pollute_user_context(true)
            .timeout_millis(500)
            .attachments(attachments)
            .cache(true)
            .avoid_null_pointer(true)
            .max_arr_length(10)
            .trace_expression(true)
            .short_circuit_disable(true)
            .build();
        assert!(opts.is_precise());
        assert!(opts.is_pollute_user_context());
        assert_eq!(opts.timeout_millis(), 500);
        assert_eq!(opts.attachments().get("k"), Some(&DataValue::Int(1)));
        assert!(opts.is_cache());
        assert!(opts.is_avoid_null_pointer());
        assert_eq!(opts.max_arr_length(), 10);
        assert!(opts.is_trace_expression());
        assert!(opts.is_short_circuit_disable());
    }

    #[test]
    fn check_arr_len_semantics() {
        assert!(QLOptions::default().check_arr_len(i32::MAX)); // -1: unlimited
        let limited = QLOptions::builder().max_arr_length(3).build();
        assert!(limited.check_arr_len(3));
        assert!(!limited.check_arr_len(4));
    }
}
