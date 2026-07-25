//! 惰性参数函数契约,对应 Java `com.alibaba.qlexpress4.runtime.function.LazyArgCustomFunction`。

use crate::runtime::function::custom_function::CustomFunction;

/// 可控制单个参数是否惰性求值的自定义函数。
/// 对应 Java: com.alibaba.qlexpress4.runtime.function.LazyArgCustomFunction
/// (职责:被标记为惰性的参数在编译期被包成 Lambda 传入,
/// 函数内按需调用才真正求值)。
///
/// 接线要点:编译期 `QvmInstructionVisitor.visitCallFunction` 对
/// `isLazyArg(i)` 为真的第 `i` 个实参生成 `LoadLambdaInstruction`
/// (Stage 3b 已落地);Rust 侧通过 [`CustomFunction::as_lazy_arg`]
/// 复现 Java 的 `instanceof` 判断。
pub trait LazyArgCustomFunction: CustomFunction {
    /// 对应 Java 方法 `isLazyArg(int argIndex)`。
    ///
    /// - `arg_index`: 函数调用中从零开始的参数下标(Java `argIndex`);
    /// - 返回 true 表示延迟求值该参数。
    ///
    /// Java 提供默认实现 `default boolean isLazyArg(int argIndex) { return true; }`,
    /// Rust 同样给出默认「全部惰性」。
    fn is_lazy_arg(&self, arg_index: usize) -> bool {
        let _ = arg_index;
        true
    }
}
