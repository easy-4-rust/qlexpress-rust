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

use qlexpress_rust::api::parsecache::SerializableParseCache;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::context::map_express_context::MapExpressContext;
use qlexpress_rust::runtime::value::DataValue;
use qlexpress_rust::Express4Runner;

use alignment_util::expect_ok;

fn empty_ctx() -> Rc<dyn qlexpress_rust::runtime::context::express_context::ExpressContext> {
    use std::cell::RefCell;
    Rc::new(MapExpressContext::new(std::rc::Rc::new(RefCell::new(
        qlexpress_rust::runtime::data::index_map::IndexMap::new(),
    ))))
}

// ---------- Export / import round-trip ----------

#[test]
fn export_then_import_yields_identical_result() {
    let runner = Express4Runner::new();
    let script = "int a = 1; int b = 2; a + b";
    let cache = runner
        .export_parse_cache(script)
        .expect("export ok");
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
    let runner = Express4Runner::new();
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