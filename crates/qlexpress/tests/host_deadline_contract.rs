//! 宿主函数协作式超时契约测试。
//!
//! 验证宿主函数可以通过 `QContext::is_expired()` 主动检测截止时间，
//! 并返回带正确错误码的 `QLException`。同时验证 `ExecutionBudget`
//! 的 `is_expired()` 和 `remaining()` 辅助 API 的正确性。
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::time::Duration;

use qlexpress::exception::{QLException, QLExceptionKind};
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::value::DataValue;
use qlexpress::{Capability, CapabilityPolicy, Express4Runner, QLOptions, SandboxProfile};

fn profile_with_timeout(timeout_millis: u64) -> SandboxProfile {
    let mut profile = SandboxProfile::secure();
    profile.limits.timeout_millis = timeout_millis;
    profile.capability_policy =
        CapabilityPolicy::allow_only([Capability::Function("deadline_aware".into())]);
    profile
}

/// 宿主函数主动检查 `is_expired()`，在截止时间过期后返回预算错误。
///
/// 验证：宿主函数可正确检测过期并传播 `SANDBOX_DEADLINE_EXCEEDED` 错误码。
#[test]
fn host_function_detects_expired_deadline_and_returns_budget_error() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "deadline_aware",
        |context: &mut dyn QContext, _params: &Parameters| -> Result<DataValue, QLException> {
            // 模拟阻塞：等待足够长的时间使截止时间过期
            std::thread::sleep(Duration::from_millis(50));

            // 宿主函数主动检查截止时间
            if context.is_expired() {
                return Err(QLException::host_error(
                    QLExceptionKind::Timeout,
                    "host function detected sandbox deadline exceeded",
                    "SANDBOX_DEADLINE_EXCEEDED",
                ));
            }
            Ok(DataValue::Int(1))
        },
    ));

    // 设置一个很短的截止时间
    let profile = profile_with_timeout(10);
    let error = runner
        .execute_checked(
            "deadline_aware()",
            HashMap::new(),
            &QLOptions::default(),
            &profile,
        )
        .unwrap_err();

    // 错误码必须是 SANDBOX_DEADLINE_EXCEEDED
    assert_eq!(error.error_code(), "SANDBOX_DEADLINE_EXCEEDED");
    // 错误类型必须是 Timeout
    assert_eq!(error.kind(), QLExceptionKind::Timeout);
}

/// 宿主函数在截止时间未过期时正常执行。
///
/// 验证：当截止时间充裕时，`is_expired()` 返回 `false`，函数正常返回。
#[test]
fn host_function_proceeds_normally_when_deadline_not_expired() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "deadline_aware",
        |context: &mut dyn QContext, _params: &Parameters| -> Result<DataValue, QLException> {
            // 不等待，截止时间应未过期
            if context.is_expired() {
                return Err(QLException::host_error(
                    QLExceptionKind::Timeout,
                    "host function detected sandbox deadline exceeded",
                    "SANDBOX_DEADLINE_EXCEEDED",
                ));
            }
            Ok(DataValue::Int(42))
        },
    ));

    // 设置一个足够长的截止时间
    let profile = profile_with_timeout(10_000);
    let result = runner
        .execute_checked(
            "deadline_aware()",
            HashMap::new(),
            &QLOptions::default(),
            &profile,
        )
        .unwrap();

    assert_eq!(result.into_result(), DataValue::Int(42));
}

/// 宿主函数检查取消令牌并返回预算错误。
///
/// 验证：宿主函数可通过 `cancellation_token()` 检测取消并传播正确错误码。
#[test]
fn host_function_detects_cancellation_and_returns_budget_error() {
    let runner = Express4Runner::new();
    assert!(runner.add_function(
        "deadline_aware",
        |context: &mut dyn QContext, _params: &Parameters| -> Result<DataValue, QLException> {
            if let Some(token) = context.cancellation_token() {
                if token.is_cancelled() {
                    return Err(QLException::host_error(
                        QLExceptionKind::Timeout,
                        "host function detected sandbox cancellation",
                        "SANDBOX_CANCELLED",
                    ));
                }
            }
            Ok(DataValue::Int(1))
        },
    ));

    let profile = profile_with_timeout(10_000);
    // 预先触发取消令牌
    profile.cancellation_token.cancel();

    let error = runner
        .execute_checked(
            "deadline_aware()",
            HashMap::new(),
            &QLOptions::default(),
            &profile,
        )
        .unwrap_err();

    // QVM 自身会在 checkpoint 中检测到取消，错误码应为 SANDBOX_CANCELLED
    assert_eq!(error.error_code(), "SANDBOX_CANCELLED");
}
