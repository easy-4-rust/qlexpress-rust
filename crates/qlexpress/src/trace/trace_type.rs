/// `TraceType` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/TraceType.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Trace point kinds, mirroring Java `TraceType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// 对应 Java: com.alibaba.qlexpress4.runtime.trace.TraceType。
pub enum TraceType {
    // parent
    /// 操作符求值节点。
    Operator,
    /// 函数调用节点。
    Function,
    /// 实例或静态方法调用节点。
    Method,
    /// 字段读取节点。
    Field,
    /// 列表字面量节点。
    List,
    /// 映射字面量节点。
    Map,
    /// 条件分支节点。
    If,
    /// 多分支选择节点。
    Switch,
    /// 返回语句节点。
    Return,
    /// 语句块节点。
    Block,
    // children
    /// 变量读取或写入节点。
    Variable,
    /// 常量值节点。
    Value,
    /// 函数定义节点。
    DefineFunction,
    /// 宏定义节点。
    DefineMacro,
    // other composite children
    /// 基础表达式节点。
    Primary,
    /// 通用语句节点。
    Statement,
}

/// 返回与 Java 语义一致的规范名称。
/// 参数：`trace_type`；返回：`&'static str`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/TraceType.java`，方法 `javaName`；Rust 侧按所有权与 `Result` 语义适配。
/// Uppercase Java enum name, used by `ExpressionTrace::to_pretty_string`.
/// 对应 Java: com.alibaba.qlexpress4.runtime.trace.TraceType#javaName。
pub fn java_name(trace_type: TraceType) -> &'static str {
    match trace_type {
        TraceType::Operator => "OPERATOR",
        TraceType::Function => "FUNCTION",
        TraceType::Method => "METHOD",
        TraceType::Field => "FIELD",
        TraceType::List => "LIST",
        TraceType::Map => "MAP",
        TraceType::If => "IF",
        TraceType::Switch => "SWITCH",
        TraceType::Return => "RETURN",
        TraceType::Block => "BLOCK",
        TraceType::Variable => "VARIABLE",
        TraceType::Value => "VALUE",
        TraceType::DefineFunction => "DEFINE_FUNCTION",
        TraceType::DefineMacro => "DEFINE_MACRO",
        TraceType::Primary => "PRIMARY",
        TraceType::Statement => "STATEMENT",
    }
}
