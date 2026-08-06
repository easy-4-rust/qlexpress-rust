//! P0 沙箱验收：预算、capability、缓存、取消与宿主调用期限。
#![allow(clippy::result_large_err)]

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

use qlexpress::operator::operator_check_strategy::OperatorCheckStrategy;
use qlexpress::{
    Capability, CheckOptions, DataValue, Express4Runner, InitOptions, QLOptions,
    QLSecurityStrategy, ResourceLimits, SandboxProfile,
};

fn profile_with(mut change: impl FnMut(&mut SandboxProfile)) -> SandboxProfile {
    let mut profile = SandboxProfile::secure();
    change(&mut profile);
    profile
}

fn execute(
    runner: &Express4Runner,
    script: &str,
    profile: &SandboxProfile,
) -> Result<DataValue, qlexpress::QLException> {
    runner
        .execute_checked(script, HashMap::new(), &QLOptions::default(), profile)
        .map(qlexpress::QLResult::into_result)
}

#[test]
fn secure_profile_keeps_java_options_defaults_unchanged() {
    let options = QLOptions::default();
    assert_eq!(options.timeout_millis(), -1);
    assert_eq!(options.max_arr_length(), -1);
    assert_eq!(
        execute(&Express4Runner::new(), "1 + 2", &SandboxProfile::secure()).unwrap(),
        DataValue::Int(3)
    );
}

#[test]
fn rejects_source_token_ast_and_instruction_budgets() {
    let runner = Express4Runner::new();

    let source_profile = profile_with(|profile| profile.limits.max_source_bytes = 3);
    assert_eq!(
        execute(&runner, "1 + 2", &source_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_SOURCE_BYTES_EXCEEDED"
    );

    let token_profile = profile_with(|profile| profile.limits.max_tokens = 2);
    assert_eq!(
        execute(&runner, "1 + 2", &token_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_TOKENS_EXCEEDED"
    );

    let depth_profile = profile_with(|profile| profile.limits.max_ast_depth = 3);
    assert_eq!(
        execute(&runner, "((((1))))", &depth_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_AST_DEPTH_EXCEEDED"
    );

    let instruction_profile = profile_with(|profile| profile.limits.max_instructions = 1);
    assert_eq!(
        execute(&runner, "a = 1; a + 2", &instruction_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_INSTRUCTIONS_EXCEEDED"
    );

    // 编译预算必须递归统计 Lambda 体，不能只看根指令数组。
    let nested_instruction_profile = profile_with(|profile| profile.limits.max_instructions = 3);
    assert_eq!(
        execute(&runner, "x -> x + 1", &nested_instruction_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_INSTRUCTIONS_EXCEEDED"
    );
}

#[test]
fn rejects_fuel_call_depth_collection_string_and_output_budgets() {
    let runner = Express4Runner::new();
    let fuel_profile = profile_with(|profile| {
        profile.limits.max_fuel = 50;
        profile.limits.timeout_millis = 5_000;
    });
    assert_eq!(
        execute(&runner, "while (true) {}", &fuel_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_FUEL_EXCEEDED"
    );
    assert_eq!(
        execute(&runner, "for (;;) {}", &fuel_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_FUEL_EXCEEDED"
    );

    let call_profile = profile_with(|profile| profile.limits.max_call_depth = 8);
    assert_eq!(
        execute(
            &runner,
            "function recurse(){ return recurse(); } recurse();",
            &call_profile
        )
        .unwrap_err()
        .error_code(),
        "SANDBOX_CALL_DEPTH_EXCEEDED"
    );

    let collection_profile = profile_with(|profile| profile.limits.max_collection_items = 2);
    assert_eq!(
        execute(&runner, "[1, 2, 3]", &collection_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_COLLECTION_ITEMS_EXCEEDED"
    );

    let string_profile = profile_with(|profile| profile.limits.max_string_bytes = 3);
    assert_eq!(
        execute(&runner, "\"abcd\"", &string_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_STRING_BYTES_EXCEEDED"
    );

    let output_profile = profile_with(|profile| profile.limits.max_output_bytes = 2);
    assert_eq!(
        execute(&runner, "\"abcd\"", &output_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_OUTPUT_BYTES_EXCEEDED"
    );
}

#[test]
fn static_check_and_capability_policy_are_mandatory() {
    let runner = Express4Runner::new();
    assert!(runner.add_function_unary("echo", |value| value));
    let denied = execute(&runner, "echo(1)", &SandboxProfile::secure()).unwrap_err();
    assert_eq!(denied.error_code(), "SANDBOX_CAPABILITY_DENIED");

    let allowed = profile_with(|profile| {
        profile.capability_policy =
            qlexpress::CapabilityPolicy::allow_only([Capability::Function("echo".into())]);
    });
    assert_eq!(
        execute(&runner, "echo(1)", &allowed).unwrap(),
        DataValue::Int(1)
    );

    let mut forbidden = HashSet::new();
    forbidden.insert("+".to_string());
    let checked = profile_with(|profile| {
        profile.check_options = CheckOptions::builder()
            .operator_check_strategy(OperatorCheckStrategy::blacklist(forbidden.clone()))
            .build();
    });
    assert_eq!(
        execute(&Express4Runner::new(), "1 + 2", &checked)
            .unwrap_err()
            .error_code(),
        qlexpress::exception::error_codes::OPERATOR_NOT_ALLOWED
    );

    runner.set_security_strategy(QLSecurityStrategy::open());
    assert_eq!(
        execute(&runner, "1", &allowed).unwrap_err().error_code(),
        "SANDBOX_NATIVE_POLICY_UNSAFE"
    );
}

#[test]
fn builtin_extension_methods_also_require_capability() {
    let runner = Express4Runner::new();
    let script = "[1, 2].map(x -> x + 1)";
    assert_eq!(
        execute(&runner, script, &SandboxProfile::secure())
            .unwrap_err()
            .error_code(),
        "SANDBOX_CAPABILITY_DENIED"
    );
    let allowed = profile_with(|profile| {
        profile.capability_policy =
            qlexpress::CapabilityPolicy::allow_only([Capability::ExtensionMethod {
                type_name: "java.util.List".into(),
                method_name: "map".into(),
            }]);
    });
    assert_eq!(
        execute(&runner, script, &allowed).unwrap(),
        DataValue::list(vec![DataValue::Int(2), DataValue::Int(3)])
    );
}

#[test]
fn cancellation_and_host_deadline_are_visible_and_enforced() {
    let runner = Express4Runner::new();
    let cancelled = SandboxProfile::secure();
    cancelled.cancellation_token.cancel();
    assert_eq!(
        execute(&runner, "1", &cancelled).unwrap_err().error_code(),
        "SANDBOX_CANCELLED"
    );

    let host_runner = Express4Runner::new();
    assert!(host_runner.add_function(
        "blocking",
        |context: &mut dyn qlexpress::runtime::qcontext::QContext,
         _parameters: &qlexpress::runtime::parameters::Parameters| {
            assert!(context.deadline().is_some());
            assert!(context.cancellation_token().is_some());
            std::thread::sleep(Duration::from_millis(30));
            Ok(DataValue::Int(1))
        }
    ));
    let blocking_profile = profile_with(|profile| {
        profile.limits.timeout_millis = 10;
        profile.capability_policy =
            qlexpress::CapabilityPolicy::allow_only([Capability::Function("blocking".into())]);
    });
    let started = Instant::now();
    let error = execute(&host_runner, "blocking()", &blocking_profile).unwrap_err();
    assert_eq!(error.error_code(), "SANDBOX_DEADLINE_EXCEEDED");
    // 同步宿主调用不能被进程内 QVM 抢占，只能在返回后检查。
    assert!(started.elapsed() >= Duration::from_millis(25));
}

#[test]
fn cache_is_tenant_bounded_lru_and_reports_statistics() {
    let runner = Express4Runner::new();
    let tenant_a = profile_with(|profile| {
        profile.tenant_id = "tenant-a".into();
        profile.compile_cache.max_entries = 3;
        profile.compile_cache.max_entries_per_tenant = 2;
    });
    let tenant_b = profile_with(|profile| {
        profile.tenant_id = "tenant-b".into();
        profile.compile_cache.max_entries = 3;
        profile.compile_cache.max_entries_per_tenant = 2;
    });

    execute(&runner, "1", &tenant_a).unwrap();
    execute(&runner, "2", &tenant_a).unwrap();
    execute(&runner, "1", &tenant_a).unwrap();
    execute(&runner, "3", &tenant_a).unwrap();
    execute(&runner, "4", &tenant_b).unwrap();

    let stats = runner.compile_cache_stats();
    assert_eq!(stats.entries, 3);
    assert!(stats.hits >= 1);
    assert!(stats.evictions >= 1);
}

#[test]
fn secure_lru_cannot_evict_java_compatible_cache_entries() {
    let runner = Express4Runner::new();
    let compatible = runner
        .parse_to_definition_with_cache("40 + 2")
        .expect("populate Java-compatible cache");
    let profile = profile_with(|profile| {
        profile.tenant_id = "bounded".into();
        profile.compile_cache.max_entries = 1;
        profile.compile_cache.max_entries_per_tenant = 1;
    });

    execute(&runner, "1", &profile).expect("first secure cache entry");
    execute(&runner, "2", &profile).expect("evict first secure cache entry");

    let compatible_again = runner
        .parse_to_definition_with_cache("40 + 2")
        .expect("Java-compatible entry must survive secure LRU eviction");
    assert!(Rc::ptr_eq(&compatible, &compatible_again));
}

#[test]
fn profile_rejects_unbounded_or_invalid_limits() {
    let runner = Express4Runner::new();
    let invalid = profile_with(|profile| {
        profile.limits = ResourceLimits {
            max_fuel: 0,
            ..ResourceLimits::default()
        };
    });
    assert_eq!(
        execute(&runner, "1", &invalid).unwrap_err().error_code(),
        "SANDBOX_INVALID_PROFILE"
    );
}

#[test]
fn input_collections_are_charged_cumulatively_and_trace_is_disabled() {
    let runner = Express4Runner::new();
    let mut context = HashMap::new();
    context.insert(
        "left".to_string(),
        DataValue::list(vec![DataValue::Int(1), DataValue::Int(2)]),
    );
    context.insert(
        "right".to_string(),
        DataValue::list(vec![DataValue::Int(3), DataValue::Int(4)]),
    );
    let collection_profile = profile_with(|profile| profile.limits.max_collection_items = 3);
    assert_eq!(
        runner
            .execute_checked("1", context, &QLOptions::default(), &collection_profile)
            .unwrap_err()
            .error_code(),
        "SANDBOX_COLLECTION_ITEMS_EXCEEDED"
    );

    let trace_runner =
        Express4Runner::with_init_options(InitOptions::builder().trace_expression(true).build());
    let trace_options = QLOptions::builder().trace_expression(true).build();
    assert_eq!(
        trace_runner
            .execute_checked(
                "1 + 2",
                HashMap::new(),
                &trace_options,
                &SandboxProfile::secure()
            )
            .unwrap_err()
            .error_code(),
        "SANDBOX_TRACE_DISABLED"
    );
}
