//! Execution results: public `QLResult` plus the VM-internal `QResult`,
//! mirroring Java `QLResult` and `runtime/QResult`.

use crate::runtime::trace::ExpressionTrace;
use crate::runtime::value::DataValue;

/// Public result of `Express4Runner.execute`, mirroring Java `QLResult`.
#[derive(Clone, Debug)]
pub struct QLResult {
    result: DataValue,
    expression_traces: Vec<ExpressionTrace>,
}

impl QLResult {
    pub fn new(result: DataValue, expression_traces: Vec<ExpressionTrace>) -> Self {
        QLResult {
            result,
            expression_traces,
        }
    }

    pub fn result(&self) -> &DataValue {
        &self.result
    }

    pub fn expression_traces(&self) -> &[ExpressionTrace] {
        &self.expression_traces
    }

    /// Convenience: consume and return just the result value.
    pub fn into_result(self) -> DataValue {
        self.result
    }
}

/// Internal result of executing one instruction, mirroring Java
/// `runtime/QResult`. The five `ResultType` cases become enum variants; the
/// jump target (Java: an int-valued `Value`) is carried by
/// [`QResult::Jump`].
#[derive(Clone, Debug, PartialEq)]
pub enum QResult {
    /// Java `ResultType.BREAK` (loop break).
    Break,
    /// Java `ResultType.CONTINUE` (loop continue; also "no return value").
    Continue,
    /// Java `ResultType.JUMP`: jump to this instruction position.
    Jump(i32),
    /// Java `ResultType.RETURN`: return from function/lambda/script.
    Return(DataValue),
    /// Java `ResultType.NEXT_INSTRUCTION`.
    NextInstruction,
}

impl QResult {
    /// Java `QResult.LOOP_BREAK_RESULT`.
    pub const LOOP_BREAK_RESULT: QResult = QResult::Break;

    /// Java `QResult.LOOP_CONTINUE_RESULT`.
    pub const LOOP_CONTINUE_RESULT: QResult = QResult::Continue;

    /// Java `QResult.NEXT_INSTRUCTION`.
    pub const NEXT_INSTRUCTION: QResult = QResult::NextInstruction;

    /// The value carried by this result, mirroring Java `getResult()`
    /// (Java stores `Value.NULL_VALUE` for non-carrying results).
    pub fn value(&self) -> DataValue {
        match self {
            QResult::Return(v) => v.clone(),
            _ => DataValue::Null,
        }
    }

    pub fn is_break(&self) -> bool {
        matches!(self, QResult::Break)
    }

    pub fn is_continue(&self) -> bool {
        matches!(self, QResult::Continue)
    }

    pub fn is_next_instruction(&self) -> bool {
        matches!(self, QResult::NextInstruction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_java_result_types() {
        assert_eq!(QResult::LOOP_BREAK_RESULT, QResult::Break);
        assert_eq!(QResult::LOOP_CONTINUE_RESULT, QResult::Continue);
        assert_eq!(QResult::NEXT_INSTRUCTION, QResult::NextInstruction);
        assert_eq!(QResult::Break.value(), DataValue::Null);
        assert_eq!(QResult::Return(DataValue::Int(3)).value(), DataValue::Int(3));
    }
}
