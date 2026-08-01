//! Java 宿主语义 testsuite fixture 的直接回放。
//!
//! 每个用例直接使用 vendored Java `.ql` 文件，而不是重写为等价字符串；宿主
//! Java API 无法逐字复刻时，通过 `alignment_util` 的 NativeRegistry/JDK 类型
//! 供应器提供 Rust 等价环境。对应 Java: `TestSuiteRunner#suiteTest`。

#![allow(clippy::result_large_err)]

mod alignment_util;

use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;

/// 执行 Java testsuite 脚本并带上与 Java `TEST_PATH` 附件相同的来源路径。
fn run_fixture(path: &str, script: &str) -> Result<DataValue, qlexpress::QLException> {
    let runner = alignment_util::suite_runner();
    let options = QLOptions::builder()
        .attachments(std::collections::HashMap::from([(
            "TEST_PATH".to_string(),
            DataValue::Str(format!("/java/{path}").into()),
        )]))
        .build();
    runner
        .execute(script, std::collections::HashMap::new(), &options)
        .map(qlexpress::QLResult::into_result)
}

/// 返回第一个无法通过的单行 `assert` 所在行，便于将 fixture 回放失败定位到
/// Java 原始脚本，而不是只报告整个文件失败。
fn first_failed_assert_line(path: &str, script: &str) -> Option<usize> {
    let mut prefix = String::new();
    for (index, line) in script.lines().enumerate() {
        prefix.push_str(line);
        prefix.push('\n');
        if line.trim_start().starts_with("assert(") && run_fixture(path, &prefix).is_err() {
            return Some(index + 1);
        }
    }
    None
}

/// 对应 Java fixture `testsuite/java/array/arr_literal.ql`。
#[test]
fn java_array_literal_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/array/arr_literal.ql");
    assert!(run_fixture("array/arr_literal.ql", script).is_ok());
}

/// 对应 Java fixture `testsuite/java/array/arr_index_out_of_bound.ql`。
#[test]
fn java_array_out_of_bound_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/array/arr_index_out_of_bound.ql");
    let error = run_fixture("array/arr_index_out_of_bound.ql", script)
        .expect_err("Java fixture must preserve INDEX_OUT_BOUND");
    assert_eq!(error.error_code(), "INDEX_OUT_BOUND");
}

/// 逐项回放 Java `testsuite/java/array` 的正常数组语义脚本。
#[test]
fn java_array_success_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "array/arr_with_init_item.ql",
            include_str!("fixtures/java-testsuite/java/array/arr_with_init_item.ql"),
        ),
        (
            "array/array_item_type_convert.ql",
            include_str!("fixtures/java-testsuite/java/array/array_item_type_convert.ql"),
        ),
        (
            "array/array_slice.ql",
            include_str!("fixtures/java-testsuite/java/array/array_slice.ql"),
        ),
        (
            "array/multi_dim_array.ql",
            include_str!("fixtures/java-testsuite/java/array/multi_dim_array.ql"),
        ),
        (
            "array/type_arr.ql",
            include_str!("fixtures/java-testsuite/java/array/type_arr.ql"),
        ),
    ];

    for (path, script) in fixtures {
        assert!(
            run_fixture(path, script).is_ok(),
            "fixture must pass: {path}"
        );
    }
}

/// 逐项回放 Java `testsuite/java/array` 的错误码脚本。
#[test]
fn java_array_error_fixtures_preserve_error_codes() {
    let fixtures = [
        (
            "array/comma_absent.ql",
            include_str!("fixtures/java-testsuite/java/array/comma_absent.ql"),
            "SYNTAX_ERROR",
        ),
        (
            "array/invalid_arr_define.ql",
            include_str!("fixtures/java-testsuite/java/array/invalid_arr_define.ql"),
            "SYNTAX_ERROR",
        ),
        (
            "array/invalid_arr_item.ql",
            include_str!("fixtures/java-testsuite/java/array/invalid_arr_item.ql"),
            "INCOMPATIBLE_ARRAY_ITEM_TYPE",
        ),
        (
            "array/invalid_arr_size_define.ql",
            include_str!("fixtures/java-testsuite/java/array/invalid_arr_size_define.ql"),
            "SYNTAX_ERROR",
        ),
        (
            "array/invalid_arr_size_type.ql",
            include_str!("fixtures/java-testsuite/java/array/invalid_arr_size_type.ql"),
            "ARRAY_SIZE_NUM_REQUIRED",
        ),
    ];

    for (path, script, expected_code) in fixtures {
        let error = run_fixture(path, script).expect_err("fixture must fail");
        assert_eq!(error.error_code(), expected_code, "fixture: {path}");
    }
}

/// 逐项回放 Java `testsuite/java/cast` 的类型转换脚本。
#[test]
fn java_cast_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "cast/assignable_cast.ql",
            include_str!("fixtures/java-testsuite/java/cast/assignable_cast.ql"),
        ),
        (
            "cast/define_local_cast.ql",
            include_str!("fixtures/java-testsuite/java/cast/define_local_cast.ql"),
        ),
        (
            "cast/object_cast.ql",
            include_str!("fixtures/java-testsuite/java/cast/object_cast.ql"),
        ),
        (
            "cast/string_cast.ql",
            include_str!("fixtures/java-testsuite/java/cast/string_cast.ql"),
        ),
    ];

    for (path, script) in fixtures {
        assert!(
            run_fixture(path, script).is_ok(),
            "fixture must pass: {path}"
        );
    }
}

/// 逐项回放 Java `testsuite/java/implicit` 的正常隐式转换脚本。
#[test]
fn java_implicit_conversion_success_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "implicit/arithmetic.ql",
            include_str!("fixtures/java-testsuite/java/implicit/arithmetic.ql"),
        ),
        (
            "implicit/assignment_basic.ql",
            include_str!("fixtures/java-testsuite/java/implicit/assignment_basic.ql"),
        ),
        (
            "implicit/assignment_extend.ql",
            include_str!("fixtures/java-testsuite/java/implicit/assignment_extend.ql"),
        ),
        (
            "implicit/function_param.ql",
            include_str!("fixtures/java-testsuite/java/implicit/function_param.ql"),
        ),
        (
            "implicit/packing.ql",
            include_str!("fixtures/java-testsuite/java/implicit/packing.ql"),
        ),
        (
            "implicit/pointer.ql",
            include_str!("fixtures/java-testsuite/java/implicit/pointer.ql"),
        ),
    ];

    for (path, script) in fixtures {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "fixture must pass: {path}; first failing assert line={:?}; {}: {}",
                first_failed_assert_line(path, script),
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 对应 Java fixture `testsuite/java/implicit/incompatible_assignment_type.ql`。
#[test]
fn java_implicit_incompatible_assignment_fixture_preserves_error_code() {
    let script =
        include_str!("fixtures/java-testsuite/java/implicit/incompatible_assignment_type.ql");
    assert!(
        run_fixture("implicit/incompatible_assignment_type.ql", script).is_ok(),
        "fixture validates its two INCOMPATIBLE_ASSIGNMENT_TYPE branches"
    );
}

/// 逐项回放 Java `testsuite/java/import` 的正常导入脚本。
#[test]
fn java_import_success_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "import/import_class.ql",
            include_str!("fixtures/java-testsuite/java/import/import_class.ql"),
        ),
        (
            "import/import_package.ql",
            include_str!("fixtures/java-testsuite/java/import/import_package.ql"),
        ),
        (
            "import/multi_import.ql",
            include_str!("fixtures/java-testsuite/java/import/multi_import.ql"),
        ),
    ];

    for (path, script) in fixtures {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 逐项回放 Java `testsuite/java/import` 的语法错误脚本。
#[test]
fn java_import_error_fixtures_preserve_error_codes() {
    let fixtures = [
        (
            "import/import_not_at_beginning.ql",
            include_str!("fixtures/java-testsuite/java/import/import_not_at_beginning.ql"),
        ),
        (
            "import/import_not_end_with_semi.ql",
            include_str!("fixtures/java-testsuite/java/import/import_not_end_with_semi.ql"),
        ),
        (
            "import/import_pack_not_end_with_semi.ql",
            include_str!("fixtures/java-testsuite/java/import/import_pack_not_end_with_semi.ql"),
        ),
        (
            "import/import_star.ql",
            include_str!("fixtures/java-testsuite/java/import/import_star.ql"),
        ),
        (
            "import/incomplete_import.ql",
            include_str!("fixtures/java-testsuite/java/import/incomplete_import.ql"),
        ),
        (
            "import/invalid_package.ql",
            include_str!("fixtures/java-testsuite/java/import/invalid_package.ql"),
        ),
        (
            "import/not_support_import_static.ql",
            include_str!("fixtures/java-testsuite/java/import/not_support_import_static.ql"),
        ),
    ];

    for (path, script) in fixtures {
        let error = run_fixture(path, script).expect_err("fixture must fail");
        assert_eq!(error.error_code(), "SYNTAX_ERROR", "fixture: {path}");
    }
}

/// 逐项回放 Java `testsuite/java/number` 的数字字面量与包装类型脚本。
#[test]
fn java_number_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "number/long_max_value.ql",
            include_str!("fixtures/java-testsuite/java/number/long_max_value.ql"),
        ),
        (
            "number/min_value_not_equal_to_hex.ql",
            include_str!("fixtures/java-testsuite/java/number/min_value_not_equal_to_hex.ql"),
        ),
        (
            "number/number_auto_type.ql",
            include_str!("fixtures/java-testsuite/java/number/number_auto_type.ql"),
        ),
        (
            "number/number_invoke.ql",
            include_str!("fixtures/java-testsuite/java/number/number_invoke.ql"),
        ),
    ];

    for (path, script) in fixtures {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 逐项回放 Java `testsuite/java/generics` 的声明解析脚本。
#[test]
fn java_generic_fixture_replay_unchanged() {
    let success = include_str!("fixtures/java-testsuite/java/generics/generics.ql");
    assert!(run_fixture("generics/generics.ql", success).is_ok());

    let invalid = include_str!("fixtures/java-testsuite/java/generics/invalid_type_bound.ql");
    let error = run_fixture("generics/invalid_type_bound.ql", invalid)
        .expect_err("invalid generic bound must be rejected");
    assert_eq!(error.error_code(), "SYNTAX_ERROR");
}

/// 对应 Java fixture `testsuite/java/for/for_each_array.ql`。
#[test]
fn java_for_each_array_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/for/for_each_array.ql");
    assert!(run_fixture("for/for_each_array.ql", script).is_ok());
}

/// 逐项回放不依赖 Java SAM 反射的 Lambda fixture。
#[test]
fn java_map_lambda_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "lambda/lambda_implicit.ql",
            include_str!("fixtures/java-testsuite/java/lambda/lambda_implicit.ql"),
        ),
        (
            "lambda/lambda_method.ql",
            include_str!("fixtures/java-testsuite/java/lambda/lambda_method.ql"),
        ),
    ];

    for (path, script) in fixtures {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 对应 Java fixture `testsuite/java/newexpr/noArgument.ql`。
#[test]
fn java_hash_map_constructor_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/newexpr/noArgument.ql");
    assert!(run_fixture("newexpr/noArgument.ql", script).is_ok());
}

/// 对应 Java fixture `testsuite/java/newexpr/new_resolver.ql` 的完整构造器
/// 重载、继承、可变参数与 Lambda 参数选择。
#[test]
fn java_constructor_overload_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/newexpr/new_resolver.ql");
    if let Err(error) = run_fixture("newexpr/new_resolver.ql", script) {
        panic!(
            "constructor overload fixture; first failing assert line={:?}; {}: {}",
            first_failed_assert_line("newexpr/new_resolver.ql", script),
            error.error_code(),
            error.reason()
        );
    }
}

/// 对应 Java fixture `testsuite/java/newexpr/no_match_constructor.ql`。
#[test]
fn java_no_suitable_constructor_fixture_preserves_error_code() {
    let script = include_str!("fixtures/java-testsuite/java/newexpr/no_match_constructor.ql");
    let error = run_fixture("newexpr/no_match_constructor.ql", script)
        .expect_err("unmatched constructor must fail");
    assert_eq!(error.error_code(), "NO_SUITABLE_CONSTRUCTOR");
}

/// 逐项回放 Java `testsuite/java/method_reference` 的内建类型方法引用。
#[test]
fn java_builtin_method_reference_fixtures_replay_unchanged() {
    let class_method =
        include_str!("fixtures/java-testsuite/java/method_reference/class_method.ql");
    if let Err(error) = run_fixture("method_reference/class_method.ql", class_method) {
        panic!(
            "class static/member method reference fixture; {}: {}",
            error.error_code(),
            error.reason()
        );
    }

    let class_object =
        include_str!("fixtures/java-testsuite/java/method_reference/class_obj_method.ql");
    if let Err(error) = run_fixture("method_reference/class_obj_method.ql", class_object) {
        panic!(
            "class object method reference fixture; {}: {}",
            error.error_code(),
            error.reason()
        );
    }

    let invalid = include_str!("fixtures/java-testsuite/java/method_reference/method_not_found.ql");
    let error = run_fixture("method_reference/method_not_found.ql", invalid)
        .expect_err("missing method reference must fail");
    assert_eq!(error.error_code(), "INVALID_ARGUMENT");
}

/// 对应 Java fixture `testsuite/java/method_reference/object_method.ql`。
#[test]
fn java_object_method_reference_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/method_reference/object_method.ql");
    if let Err(error) = run_fixture("method_reference/object_method.ql", script) {
        panic!(
            "object method reference fixture; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}

/// 回放不依赖 Java enum/JSON 专有宿主的属性访问 fixture。
#[test]
fn java_basic_property_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "property/array_length_get.ql",
            include_str!("fixtures/java-testsuite/java/property/array_length_get.ql"),
        ),
        (
            "property/class_get.ql",
            include_str!("fixtures/java-testsuite/java/property/class_get.ql"),
        ),
        (
            "property/private_member_attr_getter.ql",
            include_str!("fixtures/java-testsuite/java/property/private_member_attr_getter.ql"),
        ),
        (
            "property/private_member_attr_setter.ql",
            include_str!("fixtures/java-testsuite/java/property/private_member_attr_setter.ql"),
        ),
        (
            "property/public_static.ql",
            include_str!("fixtures/java-testsuite/java/property/public_static.ql"),
        ),
    ];

    for (path, script) in fixtures {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "property fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 逐项回放 Java enum 实例字段、静态字段和接口常量 fixture。
#[test]
fn java_enum_and_interface_property_fixtures_replay_unchanged() {
    let fixtures = [
        (
            "property/enum_get.ql",
            include_str!("fixtures/java-testsuite/java/property/enum_get.ql"),
        ),
        (
            "property/enum_member_field.ql",
            include_str!("fixtures/java-testsuite/java/property/enum_member_field.ql"),
        ),
        (
            "property/interface_const_field.ql",
            include_str!("fixtures/java-testsuite/java/property/interface_const_field.ql"),
        ),
    ];

    for (path, script) in fixtures {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "enum/interface fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }

    let missing = include_str!("fixtures/java-testsuite/java/property/enum_get_not_exist.ql");
    let error = run_fixture("property/enum_get_not_exist.ql", missing)
        .expect_err("missing enum constant must fail");
    assert_eq!(error.error_code(), "FIELD_NOT_FOUND");
}

/// 回放 Java `Parent`、公开字段及私有字段访问开关。每个脚本保持 Java 源文件
/// 原样，测试环境只提供原 Java 测试夹具所声明的能力。
#[test]
fn java_property_write_and_private_access_fixtures_replay_unchanged() {
    let normal = [
        (
            "property/null_set_invoke.ql",
            include_str!("fixtures/java-testsuite/java/property/null_set_invoke.ql"),
        ),
        (
            "property/public_member_set.ql",
            include_str!("fixtures/java-testsuite/java/property/public_member_set.ql"),
        ),
    ];
    for (path, script) in normal {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "property fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }

    let private_options = qlexpress::init_options::InitOptions::builder()
        .class_supplier(std::rc::Rc::new(alignment_util::JdkClassSupplier))
        .security_strategy(qlexpress::security::ql_security_strategy::QLSecurityStrategy::open())
        .allow_private_access(true)
        .build();
    for (path, script) in [
        (
            "property/private_member_attr_access_get.ql",
            include_str!("fixtures/java-testsuite/java/property/private_member_attr_access_get.ql"),
        ),
        (
            "property/private_member_attr_access_set.ql",
            include_str!("fixtures/java-testsuite/java/property/private_member_attr_access_set.ql"),
        ),
    ] {
        let runner = alignment_util::suite_runner_with_init_options(private_options.clone());
        let error = runner.execute(
            script,
            std::collections::HashMap::new(),
            &QLOptions::builder().build(),
        );
        assert!(
            error.is_ok(),
            "private fixture must pass: {path}; {error:?}"
        );
    }

    for (path, script) in [
        (
            "property/private_member_attr_not_access_get.ql",
            include_str!(
                "fixtures/java-testsuite/java/property/private_member_attr_not_access_get.ql"
            ),
        ),
        (
            "property/private_member_attr_not_access_set.ql",
            include_str!(
                "fixtures/java-testsuite/java/property/private_member_attr_not_access_set.ql"
            ),
        ),
    ] {
        let error = run_fixture(path, script).expect_err("private field must be unavailable");
        assert_eq!(error.error_code(), "FIELD_NOT_FOUND", "fixture: {path}");
    }
}

/// 对应 Java `testsuite/java/method/method_invoke.ql`，覆盖继承、默认接口和变参重载。
#[test]
fn java_method_invoke_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/method/method_invoke.ql");
    if let Err(error) = run_fixture("method/method_invoke.ql", script) {
        panic!(
            "method fixture must pass; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}

/// 对应 Java `TestEnum` 私有 final 字段的写入拒绝语义。
#[test]
fn java_private_final_enum_write_fixture_preserves_error_code() {
    let script =
        include_str!("fixtures/java-testsuite/java/property/private_member_set_not_accessible.ql");
    let error = run_fixture("property/private_member_set_not_accessible.ql", script)
        .expect_err("private final enum field write must fail");
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
}

/// 对应 Java `HashMap` 与字面量 Map 的相等、属性和索引访问语义。
#[test]
fn java_hash_map_equality_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/map/equal_to_hash_map.ql");
    if let Err(error) = run_fixture("map/equal_to_hash_map.ql", script) {
        panic!(
            "HashMap equality fixture must pass; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}

/// 逐项回放 Java 原始 try/catch 脚本，验证异常类型匹配与 QL 空指针转换。
#[test]
fn java_trycatch_fixtures_replay_unchanged() {
    for (path, script) in [
        (
            "trycatch/catch_java_exception.ql",
            include_str!("fixtures/java-testsuite/java/trycatch/catch_java_exception.ql"),
        ),
        (
            "trycatch/catch_operator_exception.ql",
            include_str!("fixtures/java-testsuite/java/trycatch/catch_operator_exception.ql"),
        ),
        (
            "trycatch/catch_order.ql",
            include_str!("fixtures/java-testsuite/java/trycatch/catch_order.ql"),
        ),
        (
            "trycatch/ql_npe.ql",
            include_str!("fixtures/java-testsuite/java/trycatch/ql_npe.ql"),
        ),
    ] {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "try/catch fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 对应 Java Stream + Lambda 的原始回放脚本。
#[test]
fn java_stream_lambda_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/stream/java_stream.ql");
    if let Err(error) = run_fixture("stream/java_stream.ql", script) {
        panic!(
            "Stream lambda fixture must pass; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}

/// 对应 Java Stream 的 `STObject::getPayload` 方法引用回放。
#[test]
fn java_stream_method_reference_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/stream/java_stream_method_ref.ql");
    if let Err(error) = run_fixture("stream/java_stream_method_ref.ql", script) {
        panic!(
            "Stream method-reference fixture must pass; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}

/// 回放 JDK 与用户自定义函数式接口：Java 的动态 Proxy 在 Rust 中由
/// NativeRegistry 的通用 SAM Lambda 分派等价实现，不为具体接口写特判。
#[test]
fn java_functional_interface_fixtures_replay_unchanged() {
    for (path, script) in [
        (
            "lambda/java_functional_interface.ql",
            include_str!("fixtures/java-testsuite/java/lambda/java_functional_interface.ql"),
        ),
        (
            "lambda/user_functional_interface.ql",
            include_str!("fixtures/java-testsuite/java/lambda/user_functional_interface.ql"),
        ),
    ] {
        if let Err(error) = run_fixture(path, script) {
            panic!(
                "functional-interface fixture must pass: {path}; {}: {}",
                error.error_code(),
                error.reason()
            );
        }
    }
}

/// 对应 Java Map `@class` 分类对象构造、嵌套填充及未知字段处理。
#[test]
fn java_classified_json_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/map/classified_json.ql");
    if let Err(error) = run_fixture("map/classified_json.ql", script) {
        panic!(
            "classified JSON fixture must pass; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}

/// 对应 Java fastjson2 `JSONObject` 与 `Map` 的 put/get 互操作脚本。
#[test]
fn java_jsonobject_map_interop_fixture_replays_unchanged() {
    let script = include_str!("fixtures/java-testsuite/java/property/jsonobject_vs_map_put.ql");
    if let Err(error) = run_fixture("property/jsonobject_vs_map_put.ql", script) {
        panic!(
            "JSONObject/Map fixture must pass; {}: {}",
            error.error_code(),
            error.reason()
        );
    }
}
