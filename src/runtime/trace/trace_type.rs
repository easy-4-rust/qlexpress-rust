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
