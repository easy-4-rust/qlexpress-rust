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
    pub fn builder() -> CheckOptionsBuilder {
        CheckOptionsBuilder::new()
    }

    /// Java `getCheckStrategy()`.
    pub fn check_strategy(&self) -> &OperatorCheckStrategy {
        &self.operator_check_strategy
    }

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
    pub fn new() -> Self {
        CheckOptionsBuilder {
            operator_check_strategy: OperatorCheckStrategy::allow_all(),
            disable_function_calls: false,
        }
    }

    pub fn operator_check_strategy(mut self, strategy: OperatorCheckStrategy) -> Self {
        self.operator_check_strategy = strategy;
        self
    }

    pub fn disable_function_calls(mut self, disable_function_calls: bool) -> Self {
        self.disable_function_calls = disable_function_calls;
        self
    }

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
