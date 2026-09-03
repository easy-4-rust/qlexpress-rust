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
///
/// # 协作式超时契约（宿主函数必须遵守）
///
/// 在沙箱执行（`execute_checked`）中，QVM 为每次执行设置了**墙钟截止时间**
/// 和**取消令牌**。宿主函数通过 `q_context.deadline()` 和
/// `q_context.cancellation_token()` 可以获取这些信号。
///
/// **关键约束**：QVM **无法抢占**同步阻塞的 Rust 代码。超时检测只在宿主函数
/// 返回后才会触发。因此，如果宿主函数内部执行阻塞 I/O（HTTP 请求、数据库查询、
/// 文件操作等），它**必须**自行将截止时间传播到下游客户端，并在阻塞调用前后
/// 检查是否过期。
///
/// ## 宿主函数检查清单
///
/// 1. **调用前检查**：在发起阻塞 I/O 之前，调用
///    [`QContext::is_expired()`](crate::runtime::qcontext::QContext::is_expired)
///    判断截止时间是否已过期。若已过期，立即返回错误。
///
/// 2. **传播截止时间**：将 [`QContext::deadline()`](crate::runtime::qcontext::QContext::deadline)
///    计算出的剩余时长传递给下游客户端的超时参数。例如：
///    - HTTP 客户端：设置 `timeout` 为 `context.deadline() - Instant::now()`
///    - 数据库连接：设置 `statement_timeout` 或 `socket_timeout`
///
/// 3. **检查取消令牌**：在多次阻塞调用之间，检查
///    [`QContext::cancellation_token()`](crate::runtime::qcontext::QContext::cancellation_token)
///    是否已被取消。
///
/// 4. **返回正确错误码**：检测到超时或取消时，返回带以下错误码的
///    [`QLException`]（`Timeout` 类型）：
///    - `"SANDBOX_DEADLINE_EXCEEDED"` — 截止时间已过
///    - `"SANDBOX_CANCELLED"` — 取消令牌已触发
///
/// ## 示例
///
/// ```rust,no_run
/// use qlexpress::runtime::parameters::Parameters;
/// use qlexpress::runtime::qcontext::QContext;
/// use qlexpress::runtime::value::DataValue;
/// use qlexpress::exception::{QLException, QLExceptionKind};
///
/// fn my_host_function(
///     context: &mut dyn QContext,
///     _params: &Parameters,
/// ) -> Result<DataValue, QLException> {
///     // 1. 调用前检查截止时间
///     if context.is_expired() {
///         return Err(QLException::host_error(
///             QLExceptionKind::Timeout,
///             "host function detected deadline exceeded",
///             "SANDBOX_DEADLINE_EXCEEDED",
///         ));
///     }
///
///     // 2. 传播截止时间到下游 HTTP 客户端
///     if let Some(deadline) = context.deadline() {
///         let remaining = deadline.saturating_duration_since(std::time::Instant::now());
///         // http_client.get(url).timeout(remaining).send()?;
///     }
///
///     // 3. 检查取消令牌
///     if context.cancellation_token().is_some_and(|t| t.is_cancelled()) {
///         return Err(QLException::host_error(
///             QLExceptionKind::Timeout,
///             "host function detected cancellation",
///             "SANDBOX_CANCELLED",
///         ));
///     }
///
///     Ok(DataValue::Int(42))
/// }
/// ```
pub trait CustomFunction {
    /// 对应 Java 方法 `call(QContext, Parameters)`。
    ///
    /// # 协作式超时
    ///
    /// 沙箱执行中，宿主函数**必须**遵守上述超时契约。QVM 无法抢占同步阻塞
    /// 代码，因此超时检测依赖宿主函数的主动配合。忽略截止时间的宿主函数
    /// 可能导致请求在超期后继续消耗资源，直到 QVM 在调用返回后检测到超时。
    ///
    /// 对于需要硬隔离保证的敌对输入，请使用 [`qlexpress-process`](https://docs.rs/qlexpress-process)
    /// 的 [`ProcessWorker`](https://docs.rs/qlexpress-process/latest/qlexpress_process/struct.ProcessWorker.html)
    /// 提供进程级超时和 Unix 资源限制。
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
