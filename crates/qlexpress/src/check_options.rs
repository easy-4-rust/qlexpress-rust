//! Script validation configuration, mirroring Java `CheckOptions`.

use crate::operator::operator_check_strategy::OperatorCheckStrategy;

/// 脚本静态检查选项，控制可用操作符及是否允许函数调用。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/CheckOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Validation options, mirroring Java `CheckOptions`.
#[derive(Clone, Debug, PartialEq, Eq)]
/// 对应 Java: com.alibaba.qlexpress4.CheckOptions。
pub struct CheckOptions {
    /// Operator check strategy. Default `OperatorCheckStrategy::allow_all()`.
    operator_check_strategy: OperatorCheckStrategy,
    /// Whether to disable function calls in the script. Default false.
    disable_function_calls: bool,
}

impl CheckOptions {
    /// 创建校验选项构建器。对应 Java: `CheckOptions#builder`。
    pub fn builder() -> CheckOptionsBuilder {
        CheckOptionsBuilder::new()
    }

    /// 校验 strategy。
    /// 无显式参数；返回：`&OperatorCheckStrategy`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/CheckOptions.java`，方法 `checkStrategy`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getCheckStrategy()`.
    /// 对应 Java：`CheckOptions#getCheckStrategy()`。
    pub fn check_strategy(&self) -> &OperatorCheckStrategy {
        &self.operator_check_strategy
    }

    /// 返回校验阶段是否禁止函数调用。对应 Java: `CheckOptions#isDisableFunctionCalls`。
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

/// 逐项构造 [`CheckOptions`] 的构建器。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/CheckOptions.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `CheckOptions.Builder`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.CheckOptions。
pub struct CheckOptionsBuilder {
    operator_check_strategy: OperatorCheckStrategy,
    disable_function_calls: bool,
}

impl CheckOptionsBuilder {
    /// 创建采用“允许全部操作符且允许函数调用”默认值的构建器。
    /// 对应 Java: `CheckOptions.Builder` 默认状态。
    pub fn new() -> Self {
        CheckOptionsBuilder {
            operator_check_strategy: OperatorCheckStrategy::allow_all(),
            disable_function_calls: false,
        }
    }

    /// 设置操作符校验策略并返回构建器。对应 Java: `CheckOptions.Builder#operatorCheckStrategy`。
    pub fn operator_check_strategy(mut self, strategy: OperatorCheckStrategy) -> Self {
        self.operator_check_strategy = strategy;
        self
    }

    /// 设置是否禁止函数调用并返回构建器。对应 Java: `CheckOptions.Builder#disableFunctionCalls`。
    pub fn disable_function_calls(mut self, disable_function_calls: bool) -> Self {
        self.disable_function_calls = disable_function_calls;
        self
    }

    /// 构建不可变校验选项。对应 Java: `CheckOptions.Builder#build`。
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
