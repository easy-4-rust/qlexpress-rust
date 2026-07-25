//! Execution options, mirroring Java `QLOptions` (Builder pattern).

use std::collections::HashMap;

use crate::runtime::value::DataValue;

/// Attachments carried to user-defined function/operator/macro; Java uses
/// `Map<String, Object>`, Rust uses script values.
pub type Attachments = HashMap<String, DataValue>;

/// Per-execution options, mirroring Java `QLOptions`.
#[derive(Clone, Debug)]
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
    pub fn builder() -> QLOptionsBuilder {
        QLOptionsBuilder::new()
    }

    pub fn is_precise(&self) -> bool {
        self.precise
    }

    pub fn is_pollute_user_context(&self) -> bool {
        self.pollute_user_context
    }

    pub fn timeout_millis(&self) -> i64 {
        self.timeout_millis
    }

    pub fn attachments(&self) -> &Attachments {
        &self.attachments
    }

    pub fn is_cache(&self) -> bool {
        self.cache
    }

    pub fn is_avoid_null_pointer(&self) -> bool {
        self.avoid_null_pointer
    }

    pub fn max_arr_length(&self) -> i32 {
        self.max_arr_length
    }

    /// Java `checkArrLen`: true when `new_arr_len` is within the limit
    /// (unlimited when `max_arr_length == -1`).
    pub fn check_arr_len(&self, new_arr_len: i32) -> bool {
        self.max_arr_length == -1 || new_arr_len <= self.max_arr_length
    }

    pub fn is_trace_expression(&self) -> bool {
        self.trace_expression
    }

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

/// Java `QLOptions.Builder`.
#[derive(Clone, Debug)]
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

    pub fn precise(mut self, precise: bool) -> Self {
        self.precise = precise;
        self
    }

    pub fn pollute_user_context(mut self, pollute_user_context: bool) -> Self {
        self.pollute_user_context = pollute_user_context;
        self
    }

    pub fn timeout_millis(mut self, timeout_millis: i64) -> Self {
        self.timeout_millis = timeout_millis;
        self
    }

    pub fn attachments(mut self, attachments: Attachments) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn cache(mut self, cache: bool) -> Self {
        self.cache = cache;
        self
    }

    pub fn avoid_null_pointer(mut self, avoid_null_pointer: bool) -> Self {
        self.avoid_null_pointer = avoid_null_pointer;
        self
    }

    pub fn max_arr_length(mut self, max_arr_length: i32) -> Self {
        self.max_arr_length = max_arr_length;
        self
    }

    pub fn trace_expression(mut self, trace_expression: bool) -> Self {
        self.trace_expression = trace_expression;
        self
    }

    pub fn short_circuit_disable(mut self, short_circuit_disable: bool) -> Self {
        self.short_circuit_disable = short_circuit_disable;
        self
    }

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
