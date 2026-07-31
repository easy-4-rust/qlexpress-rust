//! VM 内部指令执行结果,对应 Java `com.alibaba.qlexpress4.runtime.QResult`。
//! 职责:表示单条指令执行后的控制流结果(break/continue/jump/return/下一条)。
//! 本文件由 `ql_result.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::runtime::value::DataValue;

pub use super::result_type::ResultType;

/// 单条指令执行的内部结果。对应 Java: com.alibaba.qlexpress4.runtime.QResult
/// (五种 `ResultType` 变为枚举变体;跳转目标(Java: int 值的 Value)由
/// [`QResult::Jump`] 携带)
///
/// Internal result of executing one instruction, mirroring Java
/// `runtime/QResult`. The five `ResultType` cases become enum variants; the
/// jump target (Java: an int-valued `Value`) is carried by
/// [`QResult::Jump`].
#[derive(Clone, Debug, PartialEq)]
pub enum QResult {
    /// 循环 break。对应 Java `ResultType.BREAK`。
    /// Java `ResultType.BREAK` (loop break).
    Break,
    /// 循环 continue(同时表示「无返回值」)。对应 Java `ResultType.CONTINUE`;
    /// 与 Java `QResult` 一样仍携带一个值(块的最后一个表达式值;
    /// 裸 `continue` 时为 `Value.NULL_VALUE`)。
    /// Java `ResultType.CONTINUE` (loop continue; also "no return value").
    Continue(DataValue),
    /// 跳转到指定指令位置。对应 Java `ResultType.JUMP`。
    /// Java `ResultType.JUMP`: jump to this instruction position.
    Jump(i32),
    /// 从函数/Lambda/脚本返回。对应 Java `ResultType.RETURN`。
    /// Java `ResultType.RETURN`: return from function/lambda/script.
    Return(DataValue),
    /// 顺序执行下一条指令。对应 Java `ResultType.NEXT_INSTRUCTION`。
    /// Java `ResultType.NEXT_INSTRUCTION`.
    NextInstruction,
}

impl QResult {
    /// 循环 break 结果常量。对应 Java `QResult.LOOP_BREAK_RESULT`。
    /// Java `QResult.LOOP_BREAK_RESULT`.
    pub const LOOP_BREAK_RESULT: QResult = QResult::Break;

    /// 循环 continue 结果常量。对应 Java `QResult.LOOP_CONTINUE_RESULT`。
    /// Java `QResult.LOOP_CONTINUE_RESULT`.
    pub const LOOP_CONTINUE_RESULT: QResult = QResult::Continue(DataValue::Null);

    /// 下一条指令结果常量。对应 Java `QResult.NEXT_INSTRUCTION`。
    /// Java `QResult.NEXT_INSTRUCTION`.
    pub const NEXT_INSTRUCTION: QResult = QResult::NextInstruction;

    /// 结果携带的值。对应 Java 方法 `getResult()`
    /// (Java 对不携带值的结果存 `Value.NULL_VALUE`)。
    /// The value carried by this result, mirroring Java `getResult()`.
    pub fn value(&self) -> DataValue {
        match self {
            QResult::Return(v) | QResult::Continue(v) => v.clone(),
            _ => DataValue::Null,
        }
    }

    /// 返回当前控制流结果类别。
    ///
    /// 对应 Java：`QResult#getResultType()`。
    ///
    /// # 返回值
    /// 返回与当前枚举变体一一对应的 [`ResultType`]。
    pub fn get_result_type(&self) -> ResultType {
        match self {
            QResult::Break => ResultType::Break,
            QResult::Continue(_) => ResultType::Continue,
            QResult::Jump(_) => ResultType::Jump,
            QResult::Return(_) => ResultType::Return,
            QResult::NextInstruction => ResultType::NextInstruction,
        }
    }

    /// 是否为 break 结果。对应 Java `resultType == ResultType.BREAK` 判断。
    pub fn is_break(&self) -> bool {
        matches!(self, QResult::Break)
    }

    /// 是否为 continue 结果。对应 Java `resultType == ResultType.CONTINUE` 判断。
    pub fn is_continue(&self) -> bool {
        matches!(self, QResult::Continue(_))
    }

    /// 是否为「下一条指令」结果。对应 Java `resultType == ResultType.NEXT_INSTRUCTION` 判断。
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
        assert_eq!(
            QResult::LOOP_CONTINUE_RESULT,
            QResult::Continue(DataValue::Null)
        );
        assert_eq!(QResult::NEXT_INSTRUCTION, QResult::NextInstruction);
        assert_eq!(QResult::Break.value(), DataValue::Null);
        assert_eq!(
            QResult::Return(DataValue::Int(3)).value(),
            DataValue::Int(3)
        );
        assert_eq!(QResult::Break.get_result_type(), ResultType::Break);
        assert_eq!(
            QResult::Continue(DataValue::Null).get_result_type(),
            ResultType::Continue
        );
        assert_eq!(QResult::Jump(9).get_result_type(), ResultType::Jump);
        assert_eq!(
            QResult::Return(DataValue::Null).get_result_type(),
            ResultType::Return
        );
        assert_eq!(
            QResult::NextInstruction.get_result_type(),
            ResultType::NextInstruction
        );
    }
}
