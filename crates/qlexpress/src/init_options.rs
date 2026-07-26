//! Runner-initialization options, mirroring Java `InitOptions`.

use std::rc::Rc;

use crate::aparser::import_manager::QLImport;
use crate::aparser::interpolation_mode::InterpolationMode;
use crate::class_supplier::{ClassSupplier, DefaultClassSupplier};
use crate::security::ql_security_strategy::QLSecurityStrategy;

/// Debug info sink; Java `Consumer<String>`.
pub type DebugInfoConsumer = Rc<dyn Fn(String)>;

/// Initialization options, mirroring Java `InitOptions`.
#[derive(Clone)]
pub struct InitOptions {
    class_supplier: Rc<dyn ClassSupplier>,
    /// Default imports for scripts; Java defaults to packs `java.lang`,
    /// `java.util`, `java.math`, `java.util.stream`, `java.util.function`.
    default_import: Vec<QLImport>,
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
    /// 执行 `builder` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:17` 的 `InitOptions#builder`。
    pub fn builder() -> InitOptionsBuilder {
        InitOptionsBuilder::new()
    }

    /// 执行 `class_supplier` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:179` 的 `InitOptions#classSupplier`。
    pub fn class_supplier(&self) -> &Rc<dyn ClassSupplier> {
        &self.class_supplier
    }

    /// 执行 `default_import` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:1` 的 `InitOptions`；该方法为 Rust 同职责适配接口。
    pub fn default_import(&self) -> &[QLImport] {
        &self.default_import
    }

    /// 执行 `is_debug` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:115` 的 `InitOptions#isDebug`。
    pub fn is_debug(&self) -> bool {
        self.debug
    }

    /// 执行 `debug_info_consumer` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:194` 的 `InitOptions#debugInfoConsumer`。
    pub fn debug_info_consumer(&self) -> &DebugInfoConsumer {
        &self.debug_info_consumer
    }

    /// 执行 `security_strategy` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:199` 的 `InitOptions#securityStrategy`。
    pub fn security_strategy(&self) -> &QLSecurityStrategy {
        &self.security_strategy
    }

    /// 执行 `is_allow_private_access` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:127` 的 `InitOptions#isAllowPrivateAccess`。
    pub fn is_allow_private_access(&self) -> bool {
        self.allow_private_access
    }

    /// 执行 `interpolation_mode` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:209` 的 `InitOptions#interpolationMode`。
    pub fn interpolation_mode(&self) -> InterpolationMode {
        self.interpolation_mode
    }

    /// 执行 `is_trace_expression` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:135` 的 `InitOptions#isTraceExpression`。
    pub fn is_trace_expression(&self) -> bool {
        self.trace_expression
    }

    /// 执行 `selector_start` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:219` 的 `InitOptions#selectorStart`。
    pub fn selector_start(&self) -> &str {
        &self.selector_start
    }

    /// 执行 `selector_end` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:227` 的 `InitOptions#selectorEnd`。
    pub fn selector_end(&self) -> &str {
        &self.selector_end
    }

    /// 执行 `is_strict_new_lines` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/InitOptions.java:147` 的 `InitOptions#isStrictNewLines`。
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
            .field("default_import", &self.default_import)
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

/// Java `InitOptions.Builder`.
pub struct InitOptionsBuilder {
    class_supplier: Rc<dyn ClassSupplier>,
    default_import: Vec<QLImport>,
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
    /// 构造实例。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn new() -> Self {
        InitOptionsBuilder {
            class_supplier: Rc::new(DefaultClassSupplier::instance()),
            default_import: vec![
                QLImport::import_pack("java.lang"),
                QLImport::import_pack("java.util"),
                QLImport::import_pack("java.math"),
                QLImport::import_pack("java.util.stream"),
                QLImport::import_pack("java.util.function"),
            ],
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

    /// 执行 `class_supplier` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn class_supplier(mut self, class_supplier: Rc<dyn ClassSupplier>) -> Self {
        self.class_supplier = class_supplier;
        self
    }

    /// Java `addDefaultImport`: appends to the default import list.
    pub fn add_default_import(mut self, default_import: Vec<QLImport>) -> Self {
        self.default_import.extend(default_import);
        self
    }

    /// 执行 `debug` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// 执行 `debug_info_consumer` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn debug_info_consumer(mut self, consumer: DebugInfoConsumer) -> Self {
        self.debug_info_consumer = consumer;
        self
    }

    /// 执行 `security_strategy` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn security_strategy(mut self, security_strategy: QLSecurityStrategy) -> Self {
        self.security_strategy = security_strategy;
        self
    }

    /// 执行 `allow_private_access` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn allow_private_access(mut self, allow_private_access: bool) -> Self {
        self.allow_private_access = allow_private_access;
        self
    }

    /// 执行 `interpolation_mode` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn interpolation_mode(mut self, interpolation_mode: InterpolationMode) -> Self {
        self.interpolation_mode = interpolation_mode;
        self
    }

    /// 执行 `trace_expression` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn trace_expression(mut self, trace_expression: bool) -> Self {
        self.trace_expression = trace_expression;
        self
    }

    /// Java validates `selectorStart ∈ { "${", "$[", "#{", "#[" }` and
    /// throws `IllegalArgumentException` otherwise; Rust panics with the
    /// same message.
    pub fn selector_start(mut self, selector_start: impl Into<String>) -> Self {
        let selector_start = selector_start.into();
        assert!(
            ["${", "$[", "#{", "#["].contains(&selector_start.as_str()),
            "Custom selector start must in '${{' | '$[' | '#{{' | '#['"
        );
        self.selector_start = selector_start;
        self
    }

    /// Java validates `selectorEnd` is non-empty and throws
    /// `IllegalArgumentException` otherwise; Rust panics with the same
    /// message.
    pub fn selector_end(mut self, selector_end: impl Into<String>) -> Self {
        let selector_end = selector_end.into();
        assert!(
            !selector_end.is_empty(),
            "Custom selector end must be 1 or more characters"
        );
        self.selector_end = selector_end;
        self
    }

    /// 执行 `strict_new_lines` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
    pub fn strict_new_lines(mut self, strict_new_lines: bool) -> Self {
        self.strict_new_lines = strict_new_lines;
        self
    }

    /// 执行 `build` 公开操作。Rust 适配接口；Java 无同名对象，承接 `InitOptionsBuilder` 的同职责语义。
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
        let opts = InitOptions::builder()
            .add_default_import(vec![QLImport::import_cls("java.util.Date")])
            .debug(true)
            .allow_private_access(true)
            .interpolation_mode(InterpolationMode::Variable)
            .selector_start("#{")
            .selector_end("}]")
            .strict_new_lines(false)
            .build();
        assert_eq!(opts.default_import().len(), 6);
        assert!(opts.is_debug());
        assert!(opts.is_allow_private_access());
        assert_eq!(opts.interpolation_mode(), InterpolationMode::Variable);
        assert_eq!(opts.selector_start(), "#{");
        assert_eq!(opts.selector_end(), "}]");
        assert!(!opts.is_strict_new_lines());
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
