//! Error wrapping helpers, mirroring Java `ThrowUtils`.

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;

/// 处理 wrap throwable 对应的领域职责。
/// 参数：`err`、`error_reporter`、`err_code`、`err_msg`、`args`；返回：`QLException`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/InstanceOfOperator.java`，方法 `wrapThrowable`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `ThrowUtils.wrapThrowable(Throwable, ErrorReporter, errCode,
/// errMsg, args...)`: a `QLRuntimeException` is rethrown unchanged.
///
/// In Rust every script-side failure is already a [`QLException`] (the
/// `QLRuntimeException` analogue), so — exactly like the Java `instanceof`
/// branch — the error passes through unchanged. The extra arguments are
/// kept for call-site parity with Java and to document intent.
pub fn wrap_throwable(
    err: QLException,
    error_reporter: &dyn ErrorReporter,
    err_code: &str,
    err_msg: &str,
    args: &[String],
) -> QLException {
    // Java `wrapThrowable` 只对 QLRuntimeException 原样上抛,普通
    // RuntimeException(如 ArithmeticException)会被包装成 errCode 指定的
    // 错误。Rust 统一用 QLException 后,`ARITHMETIC_EXCEPTION` 是 Java 原生
    // ArithmeticException 的标记(见 `number_math`),此处恢复 Java 的包装
    // 语义 —— 例如 `8.0F >> 2` 应报 EXECUTE_OPERATOR_EXCEPTION。
    // (对齐测试 operator/bitwise.ql 发现。)
    if err.error_code() == crate::runtime::operator::number::number_math::ARITHMETIC_EXCEPTION {
        error_reporter.report_format(err_code, err_msg, args)
    } else {
        err
    }
}

/// 处理 report user defined exception 对应的领域职责。
/// 参数：`error_reporter`、`err`；返回：`QLException`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/util/ThrowUtils.java`，方法 `reportUserDefinedException`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `ThrowUtils.reportUserDefinedException`: `INVALID_ARGUMENT` keeps
/// its code, everything else becomes `BIZ_EXCEPTION`.
///
/// The Rust migration represents Java `UserDefineException` as a
/// [`QLException`] whose code is already `INVALID_ARGUMENT` (argument
/// errors) or another code (business errors), mirroring how the Java
/// catch sites re-report only the message.
pub fn report_user_defined_exception(
    error_reporter: &dyn ErrorReporter,
    err: &QLException,
) -> QLException {
    if err.error_code() == error_codes::INVALID_ARGUMENT {
        error_reporter.report(error_codes::INVALID_ARGUMENT, err.reason())
    } else {
        error_reporter.report(error_codes::BIZ_EXCEPTION, err.reason())
    }
}
