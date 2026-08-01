//! Runner-initialization options, mirroring Java `InitOptions`.

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use crate::aparser::import_manager::QLImport;
use crate::aparser::interpolation_mode::InterpolationMode;
use crate::class_supplier::{ClassSupplier, DefaultClassSupplier};
use crate::security::ql_security_strategy::QLSecurityStrategy;

/// `DebugInfoConsumer` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/InitOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Debug info sink; Java `Consumer<String>`.
pub type DebugInfoConsumer = Rc<dyn Fn(String)>;

/// 在初始化选项、其克隆和 runner 之间共享的默认导入列表。
///
/// Java `InitOptions#getDefaultImport()` 返回构建器创建的实际可变 List；
/// Rust 通过该句柄保留 runner 创建后继续追加导入的可见性。
pub type SharedDefaultImports = Rc<RefCell<Vec<QLImport>>>;

/// 创建 [`Express4Runner`](crate::Express4Runner) 时固定的解析、调试与安全选项。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/InitOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Initialization options, mirroring Java `InitOptions`.
#[derive(Clone)]
/// 对应 Java: com.alibaba.qlexpress4.InitOptions。
pub struct InitOptions {
    class_supplier: Rc<dyn ClassSupplier>,
    /// Default imports for scripts; Java defaults to packs `java.lang`,
    /// `java.util`, `java.math`, `java.util.stream`, `java.util.function`.
    default_import: SharedDefaultImports,
    /// Enable debug mode. Default false.
    debug: bool,
    /// Consumes all debug info; valid when `debug` is true. Defaults to
    /// stdout printing (Java `System.out::println`).
    debug_info_consumer: DebugInfoConsumer,
    /// Security strategy; Java default is isolation (no host access).
    security_strategy: QLSecurityStrategy,
    /// Allow access to private fields and methods. Default false.
    allow_private_access: bool,
    /// How to manage string interpolation. Default `Script`.
    interpolation_mode: InterpolationMode,
    /// Track execution of all expressions. Default false.
    trace_expression: bool,
    /// Interpolation selector start token, one of `"${" "$[" "#{" "#["`.
    /// Default `"${"`.
    selector_start: String,
    /// Interpolation selector end token, 1+ characters. Default `"}"`.
    selector_end: String,
    /// Strictly require a line break between two statements (semicolons may
    /// be omitted). Default true.
    strict_new_lines: bool,
}

impl InitOptions {
    /// 创建引擎初始化选项构建器。对应 Java: `InitOptions#builder`。
    pub fn builder() -> InitOptionsBuilder {
        InitOptionsBuilder::new()
    }

    /// 返回类引用供应器。对应 Java: `InitOptions#classSupplier`。
    pub fn class_supplier(&self) -> &Rc<dyn ClassSupplier> {
        &self.class_supplier
    }

    /// 返回默认导入项列表的只读借用。
    /// 对应 Java: `InitOptions#getDefaultImport` 的读取用法。
    pub fn default_import(&self) -> Ref<'_, Vec<QLImport>> {
        self.default_import.borrow()
    }

    /// 返回默认导入项列表的可变借用。
    ///
    /// Java `InitOptions#getDefaultImport()` 返回实际 List，调用方对其追加
    /// 的导入会影响已创建 runner 的后续编译。
    ///
    /// # 返回值
    ///
    /// 返回共享默认导入列表的独占借用。
    /// 对应 Java：`InitOptions#getDefaultImport()` 的可变 List 引用。
    pub fn default_import_mut(&self) -> RefMut<'_, Vec<QLImport>> {
        self.default_import.borrow_mut()
    }

    /// 返回默认导入列表的共享句柄。
    ///
    /// # 返回值
    ///
    /// 返回指向同一默认导入列表的引用计数句柄。
    /// 对应 Java：`InitOptions#getDefaultImport()` 的对象引用语义。
    pub fn shared_default_imports(&self) -> SharedDefaultImports {
        Rc::clone(&self.default_import)
    }

    /// 返回是否输出编译调试信息。对应 Java: `InitOptions#isDebug`。
    pub fn is_debug(&self) -> bool {
        self.debug
    }

    /// 返回调试信息消费者。对应 Java: `InitOptions#debugInfoConsumer`。
    pub fn debug_info_consumer(&self) -> &DebugInfoConsumer {
        &self.debug_info_consumer
    }

    /// 返回成员访问安全策略。对应 Java: `InitOptions#securityStrategy`。
    pub fn security_strategy(&self) -> &QLSecurityStrategy {
        &self.security_strategy
    }

    /// 返回是否允许访问宿主私有成员。对应 Java: `InitOptions#isAllowPrivateAccess`。
    pub fn is_allow_private_access(&self) -> bool {
        self.allow_private_access
    }

    /// 返回字符串插值模式。对应 Java: `InitOptions#interpolationMode`。
    pub fn interpolation_mode(&self) -> InterpolationMode {
        self.interpolation_mode
    }

    /// 返回引擎是否允许表达式追踪。对应 Java: `InitOptions#isTraceExpression`。
    pub fn is_trace_expression(&self) -> bool {
        self.trace_expression
    }

    /// 返回插值选择器起始标记。对应 Java: `InitOptions#selectorStart`。
    pub fn selector_start(&self) -> &str {
        &self.selector_start
    }

    /// 返回插值选择器结束标记。对应 Java: `InitOptions#selectorEnd`。
    pub fn selector_end(&self) -> &str {
        &self.selector_end
    }

    /// 返回是否严格要求语句换行分隔。对应 Java: `InitOptions#isStrictNewLines`。
    pub fn is_strict_new_lines(&self) -> bool {
        self.strict_new_lines
    }
}

impl Default for InitOptions {
    /// Java `InitOptions.DEFAULT_OPTIONS`.
    fn default() -> Self {
        InitOptions::builder().build()
    }
}

impl std::fmt::Debug for InitOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InitOptions")
            .field("default_import", &self.default_import.borrow())
            .field("debug", &self.debug)
            .field("security_strategy", &self.security_strategy)
            .field("allow_private_access", &self.allow_private_access)
            .field("interpolation_mode", &self.interpolation_mode)
            .field("trace_expression", &self.trace_expression)
            .field("selector_start", &self.selector_start)
            .field("selector_end", &self.selector_end)
            .field("strict_new_lines", &self.strict_new_lines)
            .finish_non_exhaustive()
    }
}

/// 以链式 API 构造 [`InitOptions`] 的构建器。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/InitOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `InitOptions.Builder`.
/// 对应 Java: com.alibaba.qlexpress4.InitOptions。
pub struct InitOptionsBuilder {
    class_supplier: Rc<dyn ClassSupplier>,
    default_import: SharedDefaultImports,
    debug: bool,
    debug_info_consumer: DebugInfoConsumer,
    security_strategy: QLSecurityStrategy,
    allow_private_access: bool,
    interpolation_mode: InterpolationMode,
    trace_expression: bool,
    selector_start: String,
    selector_end: String,
    strict_new_lines: bool,
}

impl InitOptionsBuilder {
    /// 创建采用 Java 默认导入、安全策略和插值设置的构建器。
    /// 对应 Java: `InitOptions.Builder`。
    pub fn new() -> Self {
        InitOptionsBuilder {
            class_supplier: Rc::new(DefaultClassSupplier::instance()),
            default_import: Rc::new(RefCell::new(vec![
                QLImport::import_pack("java.lang"),
                QLImport::import_pack("java.util"),
                QLImport::import_pack("java.math"),
                QLImport::import_pack("java.util.stream"),
                QLImport::import_pack("java.util.function"),
            ])),
            debug: false,
            debug_info_consumer: Rc::new(|s| println!("{s}")),
            security_strategy: QLSecurityStrategy::isolation(),
            allow_private_access: false,
            interpolation_mode: InterpolationMode::Script,
            trace_expression: false,
            selector_start: "${".to_string(),
            selector_end: "}".to_string(),
            strict_new_lines: true,
        }
    }

    /// 设置类引用供应器并返回构建器。对应 Java: `InitOptions.Builder#classSupplier`。
    pub fn class_supplier(mut self, class_supplier: Rc<dyn ClassSupplier>) -> Self {
        self.class_supplier = class_supplier;
        self
    }

    /// 添加或注册 default import。
    /// 参数：`default_import`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/InitOptions.java`，方法 `addDefaultImport`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `addDefaultImport`: appends to the default import list.
    /// 对应 Java：`InitOptions.Builder#addDefaultImport(List<QLImport>)`。
    pub fn add_default_import(self, default_import: Vec<QLImport>) -> Self {
        self.default_import.borrow_mut().extend(default_import);
        self
    }

    /// 使用调用方共享的默认导入列表。
    ///
    /// # 参数
    ///
    /// - `default_import`：需要由选项和 runner 持续观察的导入列表。
    ///
    /// 对应 Java：`InitOptions#getDefaultImport()` 的共享 List 引用语义。
    pub fn shared_default_imports(mut self, default_import: SharedDefaultImports) -> Self {
        self.default_import = default_import;
        self
    }

    /// 设置调试输出开关并返回构建器。对应 Java: `InitOptions.Builder#debug`。
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// 设置调试信息消费者并返回构建器。对应 Java: `InitOptions.Builder#debugInfoConsumer`。
    pub fn debug_info_consumer(mut self, consumer: DebugInfoConsumer) -> Self {
        self.debug_info_consumer = consumer;
        self
    }

    /// 设置成员访问安全策略并返回构建器。对应 Java: `InitOptions.Builder#securityStrategy`。
    pub fn security_strategy(mut self, security_strategy: QLSecurityStrategy) -> Self {
        self.security_strategy = security_strategy;
        self
    }

    /// 设置私有成员访问开关并返回构建器。对应 Java: `InitOptions.Builder#allowPrivateAccess`。
    pub fn allow_private_access(mut self, allow_private_access: bool) -> Self {
        self.allow_private_access = allow_private_access;
        self
    }

    /// 设置字符串插值模式并返回构建器。对应 Java: `InitOptions.Builder#interpolationMode`。
    pub fn interpolation_mode(mut self, interpolation_mode: InterpolationMode) -> Self {
        self.interpolation_mode = interpolation_mode;
        self
    }

    /// 设置表达式追踪能力开关并返回构建器。对应 Java: `InitOptions.Builder#traceExpression`。
    pub fn trace_expression(mut self, trace_expression: bool) -> Self {
        self.trace_expression = trace_expression;
        self
    }

    /// 设置字符串插值选择器的起始标记。
    ///
    /// # Arguments
    ///
    /// * `selector_start` - 起始标记，只允许 `${`、`$[`、`#{` 或 `#[`。
    ///
    /// # Returns
    ///
    /// 返回已更新的构建器。
    ///
    /// # Panics
    ///
    /// 当 `selector_start` 不属于允许集合时触发 panic，与 Java
    /// `IllegalArgumentException` 的失败语义保持一致。
    ///
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/InitOptions.java`，方法 `selectorStart`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java validates `selectorStart ∈ { "${", "$[", "#{", "#[" }` and
    /// throws `IllegalArgumentException` otherwise; Rust panics with the
    /// same message.
    /// 对应 Java: com.alibaba.qlexpress4.InitOptions#selectorStart。
    pub fn selector_start(mut self, selector_start: impl Into<String>) -> Self {
        let selector_start = selector_start.into();
        assert!(
            ["${", "$[", "#{", "#["].contains(&selector_start.as_str()),
            "Custom selector start must in '${{' | '$[' | '#{{' | '#['"
        );
        self.selector_start = selector_start;
        self
    }

    /// 设置字符串插值选择器的结束标记。
    ///
    /// # Arguments
    ///
    /// * `selector_end` - 非空的字符串插值结束标记。
    ///
    /// # Returns
    ///
    /// 返回已更新的构建器。
    ///
    /// # Panics
    ///
    /// 当 `selector_end` 为空时触发 panic，与 Java `IllegalArgumentException`
    /// 的失败语义保持一致。
    ///
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/InitOptions.java`，方法 `selectorEnd`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java validates `selectorEnd` is non-empty and throws
    /// `IllegalArgumentException` otherwise; Rust panics with the same
    /// message.
    /// 对应 Java: com.alibaba.qlexpress4.InitOptions#selectorEnd。
    pub fn selector_end(mut self, selector_end: impl Into<String>) -> Self {
        let selector_end = selector_end.into();
        assert!(
            !selector_end.is_empty(),
            "Custom selector end must be 1 or more characters"
        );
        self.selector_end = selector_end;
        self
    }

    /// 设置严格换行解析开关并返回构建器。对应 Java: `InitOptions.Builder#strictNewLines`。
    pub fn strict_new_lines(mut self, strict_new_lines: bool) -> Self {
        self.strict_new_lines = strict_new_lines;
        self
    }

    /// 构建不可变初始化选项。对应 Java: `InitOptions.Builder#build`。
    pub fn build(self) -> InitOptions {
        InitOptions {
            class_supplier: self.class_supplier,
            default_import: self.default_import,
            debug: self.debug,
            debug_info_consumer: self.debug_info_consumer,
            security_strategy: self.security_strategy,
            allow_private_access: self.allow_private_access,
            interpolation_mode: self.interpolation_mode,
            trace_expression: self.trace_expression,
            selector_start: self.selector_start,
            selector_end: self.selector_end,
            strict_new_lines: self.strict_new_lines,
        }
    }
}

impl Default for InitOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_java() {
        let opts = InitOptions::default();
        assert_eq!(
            opts.default_import()
                .iter()
                .map(|i| i.target())
                .collect::<Vec<_>>(),
            vec![
                "java.lang",
                "java.util",
                "java.math",
                "java.util.stream",
                "java.util.function"
            ]
        );
        assert!(!opts.is_debug());
        assert_eq!(opts.security_strategy(), &QLSecurityStrategy::Isolation);
        assert!(!opts.is_allow_private_access());
        assert_eq!(opts.interpolation_mode(), InterpolationMode::Script);
        assert!(!opts.is_trace_expression());
        assert_eq!(opts.selector_start(), "${");
        assert_eq!(opts.selector_end(), "}");
        assert!(opts.is_strict_new_lines());
    }

    #[test]
    fn builder_appends_imports_and_sets_fields() {
        let debug_lines = Rc::new(RefCell::new(Vec::new()));
        let captured_debug_lines = Rc::clone(&debug_lines);
        let opts = InitOptions::builder()
            .class_supplier(Rc::new(|name: &str| {
                (name == "com.example.Host").then(|| name.to_string())
            }))
            .add_default_import(vec![QLImport::import_cls("java.util.Date")])
            .debug(true)
            .debug_info_consumer(Rc::new(move |line| {
                captured_debug_lines.borrow_mut().push(line);
            }))
            .security_strategy(QLSecurityStrategy::open())
            .allow_private_access(true)
            .interpolation_mode(InterpolationMode::Variable)
            .trace_expression(true)
            .selector_start("#{")
            .selector_end("}]")
            .strict_new_lines(false)
            .build();
        assert_eq!(
            opts.class_supplier().load_cls("com.example.Host"),
            Some("com.example.Host".to_string())
        );
        assert_eq!(opts.class_supplier().load_cls("com.example.Missing"), None);
        assert_eq!(opts.default_import().len(), 6);
        assert!(opts.is_debug());
        (opts.debug_info_consumer())("debug-line".to_string());
        assert_eq!(debug_lines.borrow().as_slice(), ["debug-line"]);
        assert_eq!(opts.security_strategy(), &QLSecurityStrategy::Open);
        assert!(opts.is_allow_private_access());
        assert_eq!(opts.interpolation_mode(), InterpolationMode::Variable);
        assert!(opts.is_trace_expression());
        assert_eq!(opts.selector_start(), "#{");
        assert_eq!(opts.selector_end(), "}]");
        assert!(!opts.is_strict_new_lines());
    }

    #[test]
    fn cloned_options_share_mutable_default_imports_like_java_list_reference() {
        let options = InitOptions::default();
        let cloned = options.clone();

        options
            .default_import_mut()
            .push(QLImport::import_cls("com.example.LateType"));

        assert_eq!(cloned.default_import().len(), 6);
        assert_eq!(
            cloned.default_import().last().map(QLImport::target),
            Some("com.example.LateType")
        );
    }

    #[test]
    #[should_panic(expected = "Custom selector start must in")]
    fn invalid_selector_start_panics_like_java_illegal_argument() {
        let _ = InitOptions::builder().selector_start("$(").build();
    }

    #[test]
    #[should_panic(expected = "Custom selector end must be 1 or more characters")]
    fn empty_selector_end_panics_like_java_illegal_argument() {
        let _ = InitOptions::builder().selector_end("").build();
    }
}
