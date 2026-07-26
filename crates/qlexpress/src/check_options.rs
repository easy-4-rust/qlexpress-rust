//! Script validation configuration, mirroring Java `CheckOptions`.

use crate::operator::operator_check_strategy::OperatorCheckStrategy;

/// Validation options, mirroring Java `CheckOptions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckOptions {
    /// Operator check strategy. Default `OperatorCheckStrategy::allow_all()`.
    operator_check_strategy: OperatorCheckStrategy,
    /// Whether to disable function calls in the script. Default false.
    disable_function_calls: bool,
}

impl CheckOptions {
    /// 执行 `builder` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/CheckOptions.java:28` 的 `CheckOptions#builder`。
    pub fn builder() -> CheckOptionsBuilder {
        CheckOptionsBuilder::new()
    }

    /// Java `getCheckStrategy()`.
    pub fn check_strategy(&self) -> &OperatorCheckStrategy {
        &self.operator_check_strategy
    }

    /// 执行 `is_disable_function_calls` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/CheckOptions.java:39` 的 `CheckOptions#isDisableFunctionCalls`。
    pub fn is_disable_function_calls(&self) -> bool {
        self.disable_function_calls
    }
}

impl Default for CheckOptions {
    /// Java `CheckOptions.DEFAULT_OPTIONS`.
    fn default() -> Self {
        CheckOptions::builder().build()
    }
}

/// Java `CheckOptions.Builder`.
#[derive(Clone, Debug)]
pub struct CheckOptionsBuilder {
    operator_check_strategy: OperatorCheckStrategy,
    disable_function_calls: bool,
}

impl CheckOptionsBuilder {
    /// 构造实例。Rust 适配接口；Java 无同名对象，承接 `CheckOptionsBuilder` 的同职责语义。
    pub fn new() -> Self {
        CheckOptionsBuilder {
            operator_check_strategy: OperatorCheckStrategy::allow_all(),
            disable_function_calls: false,
        }
    }

    /// 执行 `operator_check_strategy` 公开操作。Rust 适配接口；Java 无同名对象，承接 `CheckOptionsBuilder` 的同职责语义。
    pub fn operator_check_strategy(mut self, strategy: OperatorCheckStrategy) -> Self {
        self.operator_check_strategy = strategy;
        self
    }

    /// 执行 `disable_function_calls` 公开操作。Rust 适配接口；Java 无同名对象，承接 `CheckOptionsBuilder` 的同职责语义。
    pub fn disable_function_calls(mut self, disable_function_calls: bool) -> Self {
        self.disable_function_calls = disable_function_calls;
        self
    }

    /// 执行 `build` 公开操作。Rust 适配接口；Java 无同名对象，承接 `CheckOptionsBuilder` 的同职责语义。
    pub fn build(self) -> CheckOptions {
        CheckOptions {
            operator_check_strategy: self.operator_check_strategy,
            disable_function_calls: self.disable_function_calls,
        }
    }
}

impl Default for CheckOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn defaults_match_java() {
        let opts = CheckOptions::default();
        assert_eq!(opts.check_strategy(), &OperatorCheckStrategy::AllowAll);
        assert!(!opts.is_disable_function_calls());
    }

    #[test]
    fn builder_overrides() {
        let forbidden: HashSet<String> = ["=".to_string()].into_iter().collect();
        let opts = CheckOptions::builder()
            .operator_check_strategy(OperatorCheckStrategy::blacklist(forbidden))
            .disable_function_calls(true)
            .build();
        assert!(!opts.check_strategy().is_allowed("="));
        assert!(opts.is_disable_function_calls());
    }
}
