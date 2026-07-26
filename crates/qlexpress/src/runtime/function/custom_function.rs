//! 用户自定义函数契约,对应 Java `com.alibaba.qlexpress4.runtime.function.CustomFunction`。

use crate::exception::QLException;
use crate::runtime::function::lazy_arg_custom_function::LazyArgCustomFunction;
use crate::runtime::parameters::Parameters;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::DataValue;

/// 脚本可调用函数。对应 Java: com.alibaba.qlexpress4.runtime.function.CustomFunction
/// (职责:宿主向脚本注册函数的契约;`Express4Runner.addFunction` 的注册类型)。
///
/// Java 签名 `Object call(QContext qContext, Parameters parameters) throws Throwable`;
/// `throws Throwable` 允许抛 `UserDefineException` 携带自定义错误消息,
/// Rust 统一为 `Result<_, QLException>`。
pub trait CustomFunction {
    /// 对应 Java 方法 `call(QContext, Parameters)`。
    ///
    /// - `q_context`: 当前脚本运行上下文(Java `QContext`);
    /// - `parameters`: 调用参数(Java `Parameters`,下标越界按 Java 语义取 `null`);
    /// - 返回函数结果(Java `Object` → Rust [`DataValue`])。
    fn call(
        &self,
        q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException>;

    /// 向下转型钩子,对应 Java `QvmInstructionVisitor.visitCallFunction` 中的
    /// `customFunction instanceof LazyArgCustomFunction` 判断。
    /// 非惰性参数函数返回 `None`(Java `instanceof` 为 false)。
    fn as_lazy_arg(&self) -> Option<&dyn LazyArgCustomFunction> {
        None
    }
}

/// 让闭包/函数指针直接充当脚本函数,对应 Java 中以 lambda 实现
/// `CustomFunction` 函数式接口的写法(如 `Express4Runner.addFunction`
/// 各重载内部的 `(qContext, parameters) -> ...`)。
impl<F> CustomFunction for F
where
    F: Fn(&mut dyn QContext, &Parameters) -> Result<DataValue, QLException>,
{
    fn call(
        &self,
        q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        (self)(q_context, parameters)
    }
}
