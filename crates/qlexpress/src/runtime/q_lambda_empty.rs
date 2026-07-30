//! 空 Lambda,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaEmpty`。
//! 职责:空调用体,`call` 直接返回 `QResult.NEXT_INSTRUCTION`
//! (调用语义在 [`crate::runtime::qlambda::QLambda`] 的 `Empty` 变体分支中实现,与原类一致)。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::QLException;
use crate::runtime::q_result::QResult;
use crate::runtime::value::DataValue;

/// 空 Lambda。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaEmpty
/// (单例 `INSTANCE`;Rust 以 unit 结构体承载,作为 `QLambda::Empty` 的负载)
pub struct QLambdaEmpty;

impl QLambdaEmpty {
    /// 单例。对应 Java `QLambdaEmpty.INSTANCE`。
    /// Java `QLambdaEmpty.INSTANCE`.
    pub const INSTANCE: QLambdaEmpty = QLambdaEmpty;

    /// 调用空 Lambda，忽略全部实参并继续执行下一条指令。
    ///
    /// 对应 Java：`QLambdaEmpty#call(Object...)`。
    ///
    /// # 参数
    ///
    /// - `params`：调用者传入的任意参数；空 Lambda 不读取这些值。
    ///
    /// # 返回值
    ///
    /// 始终返回 [`QResult::NEXT_INSTRUCTION`]。
    pub fn call(&self, _params: &[DataValue]) -> Result<QResult, QLException> {
        Ok(QResult::NEXT_INSTRUCTION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::q_result::ResultType;

    /// SOURCE_PARITY: QLambdaEmpty#call(Object...)。
    #[test]
    fn call_ignores_arguments_and_returns_next_instruction() {
        let result = QLambdaEmpty::INSTANCE
            .call(&[DataValue::Int(1), DataValue::Str("ignored".into())])
            .expect("empty lambda must not fail");

        assert_eq!(result.get_result_type(), ResultType::NextInstruction);
        assert_eq!(result.value(), DataValue::Null);
    }
}
