//! Stage 6 parse-cache alignment tests.
//!
//! Mirrors Java `SerializableParseCacheTest` (8 cases). The Rust side
//! supports the JSON round-trip via `SerializableParseCacheExporter`
//! / `Importer` and re-execution through `Express4Runner`.
//!
//! These tests lock down the round-trip contract end-to-end so that
//! any change to the serializer breaks the suite visibly.

#![allow(clippy::result_large_err)]

mod alignment_util;

use std::rc::Rc;

use qlexpress::api::parsecache::SerializableParseCache;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::context::map_express_context::MapExpressContext;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

fn empty_ctx() -> Rc<dyn qlexpress::runtime::context::express_context::ExpressContext> {
    use std::cell::RefCell;
    Rc::new(MapExpressContext::new(std::rc::Rc::new(RefCell::new(
        qlexpress::runtime::data::index_map::IndexMap::new(),
    ))))
}

// ---------- Export / import round-trip ----------

#[test]
fn export_then_import_yields_identical_result() {
    let runner = Express4Runner::new();
    let script = "int a = 1; int b = 2; a + b";
    let cache = runner.export_parse_cache(script).expect("export ok");
    let json = serde_json::to_string(&cache).expect("serialize");
    let cache2: SerializableParseCache = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(cache.script.as_deref(), cache2.script.as_deref());
    let result = runner
        .execute_with_cache(&cache2, empty_ctx(), &QLOptions::builder().build())
        .expect("execute with cache")
        .into_result();
    assert_eq!(result, DataValue::Long(3));
}

// ---------- Reuse loaded cache ----------

#[test]
fn loaded_cache_executes_independently() {
    let runner = Express4Runner::new();
    let script = "1 + 2 + 3";
    let cache = runner.export_parse_cache(script).expect("export");
    let loaded = runner.import_parse_cache(&cache).expect("load");
    let r1 = runner
        .execute_with_loaded_cache(&loaded, empty_ctx(), &QLOptions::builder().build())
        .expect("exec 1")
        .into_result();
    let r2 = runner
        .execute_with_loaded_cache(&loaded, empty_ctx(), &QLOptions::builder().build())
        .expect("exec 2")
        .into_result();
    assert_eq!(r1, DataValue::Long(6));
    assert_eq!(r2, DataValue::Long(6));
}

// ---------- Functions / loops round-trip ----------

#[test]
fn function_and_loop_round_trip() {
    let runner = Express4Runner::new();
    let script = "int total = 0; for (int i = 1; i <= 5; i = i + 1) { total = total + i; } total";
    let cache = runner.export_parse_cache(script).expect("export");
    let json = serde_json::to_string(&cache).expect("serialize");
    let cache2: SerializableParseCache = serde_json::from_str(&json).expect("deserialize");
    let result = runner
        .execute_with_cache(&cache2, empty_ctx(), &QLOptions::builder().build())
        .expect("exec")
        .into_result();
    assert_eq!(result, DataValue::Long(15));
}

// ---------- Collections round-trip ----------

#[test]
fn collection_round_trip() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let script = "m = {a: 1, b: 2}; l = [m.a, m.b, 3]; l.size() + m.a + m.b";
    let cache = runner.export_parse_cache(script).expect("export");
    let json = serde_json::to_string(&cache).expect("serialize");
    let cache2: SerializableParseCache = serde_json::from_str(&json).expect("deserialize");
    let result = runner
        .execute_with_cache(&cache2, empty_ctx(), &QLOptions::builder().build())
        .expect("exec")
        .into_result();
    assert_eq!(result, DataValue::Long(6));
}

// ---------- Custom operator round-trip ----------

#[test]
fn custom_operator_round_trip() {
    let mut runner = Express4Runner::new();
    runner.add_operator_bi("join", |a, b| {
        DataValue::Str(format!("{}|{}", a.string_value_of(), b.string_value_of()))
    });
    let script = "1 join 2";
    let cache = runner.export_parse_cache(script).expect("export");
    let json = serde_json::to_string(&cache).expect("serialize");
    let cache2: SerializableParseCache = serde_json::from_str(&json).expect("deserialize");
    let result = runner
        .execute_with_cache(&cache2, empty_ctx(), &QLOptions::builder().build())
        .expect("exec")
        .into_result();
    assert_eq!(result, DataValue::Str("1|2".to_string()));
}

// ---------- Error on incompatible runner identity ----------

#[test]
fn cache_imported_to_other_runner_works() {
    // execute_with_cache 内部调用 import_parse_cache 将 cache 绑定到
    // 当前 runner identity,因此跨 runner 使用也能成功。
    // Java 端 execute(SerializableParseCache) 也是先 load 再执行。
    let runner_a = Express4Runner::new();
    let runner_b = Express4Runner::new();
    let cache = runner_a.export_parse_cache("1 + 1").expect("export");
    let result = runner_b.execute_with_cache(&cache, empty_ctx(), &QLOptions::builder().build());
    assert!(result.is_ok(), "imported cache should work on any runner");
}

#[test]
fn instruction_family_matrix_survives_json_round_trip() {
    // Rust serde/指令枚举是 Java ObjectMapper 模型的替代组件。该矩阵不是
    // 为覆盖率拼行数，而是保证每个高风险控制流/集合/调用指令族在 JSON
    // 往返后仍与直接编译执行产生完全相同的公开结果。
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let scripts = [
        "a = 1; a += 2; ++a; a--",
        "if (false) { 10 } else if (true) { 20 } else { 30 }",
        "sum = 0; i = 0; while (i < 4) { i++; if (i == 2) { continue; }\nsum += i; }\nsum",
        "sum = 0; for (item : [1, 2, 3]) { sum += item; } sum",
        "value = 2; switch (value) { case 1 -> 10 case 2 -> 20 default -> 30 }",
        "try { throw 'boom'; } catch (error) { 42 } finally { marker = 1; }",
        "twice = (x) -> { return x * 2; }; twice(9)",
        "\"value=${1 + 2}\"",
        "values = [1, 2, 3, 4]; values[1:3]",
        "int[] values = {1, 2, 3}; values[1]",
        "map = {'a': 1}; map.a = 7; map.a",
        "false && (1 / 0)",
    ];

    for script in scripts {
        let direct = runner
            .execute(
                script,
                std::collections::HashMap::new(),
                &QLOptions::builder().build(),
            )
            .unwrap_or_else(|error| panic!("direct execution failed for {script:?}: {error}"))
            .into_result();
        let exported = runner
            .export_parse_cache(script)
            .unwrap_or_else(|error| panic!("export failed for {script:?}: {error}"));
        let json = serde_json::to_vec(&exported).expect("serialize parse cache");
        let imported: SerializableParseCache =
            serde_json::from_slice(&json).expect("deserialize parse cache");
        let replayed = runner
            .execute_with_cache(&imported, empty_ctx(), &QLOptions::builder().build())
            .unwrap_or_else(|error| panic!("cached execution failed for {script:?}: {error}"))
            .into_result();
        assert_eq!(
            replayed, direct,
            "parse-cache semantic drift for {script:?}"
        );
    }
}
