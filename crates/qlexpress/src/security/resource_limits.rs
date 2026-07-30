//! 不可信脚本的资源预算。

/// `execute_checked` 在解析、编译和执行阶段共享的有限资源预算。
///
/// 该对象是 Rust 安全增强，不改变 Java `QLOptions` 的默认兼容语义。
/// `SandboxProfile::default()` 使用这里的保守默认值；所有字段必须大于零。
/// 对应 Java: 无（Rust 安全增强，用于约束不可信规则的资源消耗）。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceLimits {
    /// UTF-8 源码最大字节数。
    pub max_source_bytes: usize,
    /// 词法 Token 最大数量。
    pub max_tokens: usize,
    /// AST 最大层级。
    pub max_ast_depth: usize,
    /// AST 节点最大数量。
    pub max_ast_nodes: usize,
    /// 单份编译产物最大 QVM 指令数。
    pub max_instructions: usize,
    /// 单次执行最多取指次数。
    pub max_fuel: u64,
    /// 脚本 Lambda 与宿主调用合计最大嵌套深度。
    pub max_call_depth: usize,
    /// 单次执行可创建或从宿主接收的集合元素累计预算。
    pub max_collection_items: usize,
    /// 任意脚本字符串最大 UTF-8 字节数。
    pub max_string_bytes: usize,
    /// 最终输出值的估算最大字节数。
    pub max_output_bytes: usize,
    /// 整个检查、编译和执行过程的墙钟时间上限。
    pub timeout_millis: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 256 * 1024,
            max_tokens: 50_000,
            max_ast_depth: 256,
            max_ast_nodes: 100_000,
            max_instructions: 100_000,
            max_fuel: 1_000_000,
            max_call_depth: 128,
            max_collection_items: 100_000,
            max_string_bytes: 1024 * 1024,
            max_output_bytes: 1024 * 1024,
            timeout_millis: 1_000,
        }
    }
}

impl ResourceLimits {
    /// 校验所有预算均为有限正数。
    ///
    /// # Returns
    ///
    /// 所有字段均大于零时返回 `Ok(())`，否则返回稳定的配置错误原因。
    ///
    /// # Errors
    ///
    /// 任一限制为零时返回错误；安全入口不会把零解释为“不限制”。
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_source_bytes == 0
            || self.max_tokens == 0
            || self.max_ast_depth == 0
            || self.max_ast_nodes == 0
            || self.max_instructions == 0
            || self.max_fuel == 0
            || self.max_call_depth == 0
            || self.max_collection_items == 0
            || self.max_string_bytes == 0
            || self.max_output_bytes == 0
            || self.timeout_millis == 0
        {
            return Err("sandbox resource limits must all be greater than zero");
        }
        Ok(())
    }
}
