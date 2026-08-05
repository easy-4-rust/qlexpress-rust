//! Execution options, mirroring Java `QLOptions` (Builder pattern).

use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::value::DataValue;

/// `Attachments` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/QLOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Attachments carried to user-defined function/operator/macro; Java uses
/// `Map<String, Object>`, Rust uses script values.
pub type Attachments = HashMap<String, DataValue>;

/// 在选项、全局作用域和 QVM 运行时之间共享的附件 Map。
///
/// Java `QLOptions.Builder#attachments(Map)` 保存调用方传入的同一个 Map
/// 引用；Rust 通过 `Rc<RefCell<_>>` 保留相同的单线程引用与可变性语义。
/// 对应 Java: `com.alibaba.qlexpress4.QLOptions#getAttachments()` 返回的 Map 引用。
pub type SharedAttachments = Rc<RefCell<Attachments>>;

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
    attachments: SharedAttachments,
    /// 默认附件来自 Java `Collections.emptyMap()`，不可修改；调用方显式
    /// 提供附件 Map 后才允许通过 [`Self::attachments_mut`] 写入。
    attachments_mutable: bool,
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

    /// 返回传递给自定义扩展的只读附件。对应 Java
    /// `QLOptions#getAttachments()` 的读取用法。
    pub fn attachments(&self) -> Ref<'_, Attachments> {
        self.attachments.borrow()
    }

    /// 返回可修改的附件 Map。对应 Java
    /// `QLOptions#getAttachments()` 返回可变 Map 后的写入用法。
    ///
    /// # 返回值
    ///
    /// 返回附件表的独占借用；该借用存续期间不能再次借用附件。
    pub fn attachments_mut(&self) -> RefMut<'_, Attachments> {
        assert!(
            self.attachments_mutable,
            "UnsupportedOperationException: default attachments map is immutable"
        );
        self.attachments.borrow_mut()
    }

    /// 返回附件表的共享句柄，供运行时与已创建 Lambda 保留引用语义。
    ///
    /// # 返回值
    ///
    /// 返回指向同一附件表的引用计数句柄。
    /// 对应 Java：`QLOptions#getAttachments()` 的 Map 引用语义。
    pub fn shared_attachments(&self) -> SharedAttachments {
        Rc::clone(&self.attachments)
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
    /// 对应 Java：`QLOptions#checkArrLen(int)`。
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
    attachments: SharedAttachments,
    attachments_mutable: bool,
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
            attachments: Rc::new(RefCell::new(HashMap::new())),
            attachments_mutable: false,
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
        self.attachments = Rc::new(RefCell::new(attachments));
        self.attachments_mutable = true;
        self
    }

    /// 使用共享附件 Map 构建选项。
    ///
    /// 对应 Java `QLOptions.Builder#attachments(Map)` 保留调用方 Map 引用的
    /// 行为；调用方后续通过同一句柄修改 Map 时，执行和既有 Lambda 均可见。
    ///
    /// # 参数
    ///
    /// - `attachments`：需要与选项和运行时共享的附件表。
    pub fn shared_attachments(mut self, attachments: SharedAttachments) -> Self {
        self.attachments = attachments;
        self.attachments_mutable = true;
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
            attachments_mutable: self.attachments_mutable,
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
    fn cloned_options_share_the_java_attachment_map_identity() {
        let opts = QLOptions::builder().attachments(HashMap::new()).build();
        let cloned = opts.clone();

        opts.attachments_mut()
            .insert("late".to_string(), DataValue::Int(42));

        assert_eq!(cloned.attachments().get("late"), Some(&DataValue::Int(42)));
    }

    #[test]
    fn default_attachment_map_is_immutable_like_java_collections_empty_map() {
        let options = QLOptions::default();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            options
                .attachments_mut()
                .insert("forbidden".to_string(), DataValue::Int(1));
        }));

        assert!(result.is_err());
        assert!(options.attachments().is_empty());
    }

    #[test]
    fn check_arr_len_semantics() {
        assert!(QLOptions::default().check_arr_len(i32::MAX)); // -1: unlimited
        let limited = QLOptions::builder().max_arr_length(3).build();
        assert!(limited.check_arr_len(3));
        assert!(!limited.check_arr_len(4));
    }
}
