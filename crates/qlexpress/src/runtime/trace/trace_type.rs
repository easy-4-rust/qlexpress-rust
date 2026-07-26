/// `TraceType` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/TraceType.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Trace point kinds, mirroring Java `TraceType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TraceType {
    // parent
    Operator,
    Function,
    Method,
    Field,
    List,
    Map,
    If,
    Switch,
    Return,
    Block,
    // children
    Variable,
    Value,
    DefineFunction,
    DefineMacro,
    // other composite children
    Primary,
    Statement,
}

/// 处理 java name 对应的领域职责。
/// 参数：`trace_type`；返回：`&'static str`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/trace/TraceType.java`，方法 `javaName`；Rust 侧按所有权与 `Result` 语义适配。
/// Uppercase Java enum name, used by `ExpressionTrace::to_pretty_string`.
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
