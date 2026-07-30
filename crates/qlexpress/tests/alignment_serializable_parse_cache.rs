//! 逐项对齐 Java `api/parsecache/SerializableParseCacheTest` 的 8 个测试方法。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::compile_time_function::{CodeGenerator, CompileTimeFunction};
use qlexpress::aparser::operator_factory::OperatorFactory;
use qlexpress::aparser::syntax_tree_factory::Node;
use qlexpress::api::parsecache::{SerializableInstruction, SerializableParseCache};
use qlexpress::exception::error_codes;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::ql_precedences;
use qlexpress::runtime::context::{ExpressContext, MapExpressContext};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::instruction::CallConstInstruction;
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::qlambda::QLambda;
use qlexpress::runtime::qlambda_empty::QLambdaEmpty;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::Express4Runner;

fn context(entries: &[(&str, DataValue)]) -> Rc<dyn ExpressContext> {
    let entries = entries
        .iter()
        .map(|(key, value)| (DataValue::Str((*key).to_string()), value.clone()))
        .collect();
    Rc::new(MapExpressContext::new(Rc::new(RefCell::new(
        IndexMap::from_entries(entries),
    ))))
}

fn json_round_trip(cache: &SerializableParseCache) -> SerializableParseCache {
    serde_json::from_str(&serde_json::to_string(cache).expect("serialize cache"))
        .expect("deserialize cache")
}

fn assert_number(value: &DataValue, expected: i64) {
    match value {
        DataValue::Int(actual) => assert_eq!(i64::from(*actual), expected),
        DataValue::Long(actual) => assert_eq!(*actual, expected),
        other => panic!("expected integer {expected}, got {other:?}"),
    }
}

fn find_instruction_mut<'a>(
    cache: &'a mut SerializableParseCache,
    opcode: &str,
) -> &'a mut SerializableInstruction {
    cache
        .main
        .as_mut()
        .and_then(|main| main.instructions.as_mut())
        .and_then(|instructions| {
            instructions
                .iter_mut()
                .find(|instruction| instruction.opcode.as_deref() == Some(opcode))
        })
        .unwrap_or_else(|| panic!("instruction not found: {opcode}"))
}

fn assert_parse_cache_error(
    runner: &Express4Runner,
    cache: &SerializableParseCache,
    expected: &str,
) {
    let error = match runner.import_parse_cache(cache) {
        Ok(_) => panic!("invalid cache must be rejected"),
        Err(error) => error,
    };
    assert_eq!(error.error_code(), expected);
    assert!(!error.diagnostic().code().is_empty());
}

/// Java `SerializableParseCacheTest#jsonRoundTripImportAndExecute`。
#[test]
fn java_json_round_trip_import_and_execute() {
    let producer = Express4Runner::new();
    let cache = producer
        .export_parse_cache("price * count")
        .expect("export cache");
    let parsed = json_round_trip(&cache);
    let consumer = Express4Runner::new();
    let result = consumer
        .execute_with_cache(
            &parsed,
            context(&[("price", DataValue::Int(5)), ("count", DataValue::Int(3))]),
            &QLOptions::default(),
        )
        .expect("execute imported cache");
    assert_number(result.result(), 15);

    assert_eq!(cache.model_version, 1);
    assert!(cache.script_hash.is_some());
    let main = cache.main.as_ref().expect("main definition");
    assert_eq!(main.name.as_deref(), Some("main"));
    let instructions = main.instructions.as_ref().expect("main instructions");
    assert!(!instructions.is_empty());
    assert!(instructions[0].source.is_some());
}

/// Java `SerializableParseCacheTest#loadedParseCacheCanBeReusedOnSameRunner`。
#[test]
fn java_loaded_parse_cache_can_be_reused_on_same_runner() {
    let producer = Express4Runner::new();
    let cache = producer
        .export_parse_cache("x * 2 + 1")
        .expect("export cache");
    let consumer = Express4Runner::new();
    let loaded = consumer.import_parse_cache(&cache).expect("load cache");

    let first = consumer
        .execute_with_loaded_cache(
            &loaded,
            context(&[("x", DataValue::Int(3))]),
            &QLOptions::default(),
        )
        .expect("first execution");
    assert_number(first.result(), 7);
    let second = consumer
        .execute_with_loaded_cache(
            &loaded,
            context(&[("x", DataValue::Int(5))]),
            &QLOptions::default(),
        )
        .expect("second execution");
    assert_number(second.result(), 11);
    assert_eq!(cache.script_hash.as_deref(), loaded.get_script_hash());
}

/// Java `SerializableParseCacheTest#runtimeErrorLocationSurvivesJsonRoundTrip`。
#[test]
fn java_runtime_error_location_survives_json_round_trip() {
    let script = "a = 1\nmissing(1, 2)";
    let producer = Express4Runner::new();
    let parsed = json_round_trip(&producer.export_parse_cache(script).expect("export cache"));
    let error = Express4Runner::new()
        .execute_with_cache(&parsed, context(&[]), &QLOptions::default())
        .expect_err("missing function must fail");
    assert_eq!(error.error_code(), error_codes::FUNCTION_NOT_FOUND);
    assert_eq!(error.line_no(), 2);
    assert_eq!(error.col_no(), 1);
    assert_eq!(error.err_lexeme(), "missing");
    assert_eq!(error.diagnostic().range().start().line(), 1);
    assert_eq!(error.diagnostic().range().start().character(), 0);
}

/// Java `SerializableParseCacheTest#functionsLoopsCollectionsAndCustomOperatorRoundTrip`。
#[test]
fn java_functions_loops_collections_and_custom_operator_round_trip() {
    fn register_plus2(runner: &mut Express4Runner) {
        assert!(runner.add_operator_with_precedence(
            "plus2",
            Rc::new(|left: &QValue, right: &QValue| {
                let DataValue::Int(left) = left.get() else {
                    unreachable!("left operand is int")
                };
                let DataValue::Int(right) = right.get() else {
                    unreachable!("right operand is int")
                };
                Ok(DataValue::Int(left + right + 2))
            }),
            ql_precedences::ADD,
        ));
    }

    let script = concat!(
        "function add(int a, int b) {\n",
        "  return a plus2 b\n",
        "}\n",
        "total = 0\n",
        "for (ele: [1, 2, 3]) {\n",
        "  total = total + ele\n",
        "}\n",
        "i = 0\n",
        "while (i < 2) {\n",
        "  total = total + i\n",
        "  i = i + 1\n",
        "}\n",
        "m = {name: \"QL\", value: total}\n",
        "add(m.value, 4)"
    );
    let mut producer = Express4Runner::new();
    register_plus2(&mut producer);
    let parsed = json_round_trip(&producer.export_parse_cache(script).expect("export cache"));
    let mut consumer = Express4Runner::new();
    register_plus2(&mut consumer);
    let result = consumer
        .execute_with_cache(&parsed, context(&[]), &QLOptions::default())
        .expect("execute cache");
    assert_number(result.result(), 13);
}

/// Java `SerializableParseCacheTest#addFunctionsDefinedInSerializableCache`。
#[test]
fn java_add_functions_defined_in_serializable_cache() {
    let producer = Express4Runner::new();
    let parsed = json_round_trip(
        &producer
            .export_parse_cache(concat!(
                "base = seed\n",
                "function remoteAdd(a, b) {\n  return a + b\n}\n",
                "function capturedBase() {\n  return base\n}\n"
            ))
            .expect("export cache"),
    );
    let consumer = Express4Runner::new();
    let seed_context = context(&[("seed", DataValue::Int(9))]);
    let added = consumer
        .add_functions_defined_in_cache(&parsed, Rc::clone(&seed_context), &QLOptions::default())
        .expect("register cached functions");
    assert_eq!(added.get_succ().len(), 2);
    assert!(added.get_fail().is_empty());
    assert_number(
        consumer
            .execute("remoteAdd(3, 4)", HashMap::new(), &QLOptions::default())
            .expect("remoteAdd")
            .result(),
        7,
    );
    assert_number(
        consumer
            .execute("capturedBase()", HashMap::new(), &QLOptions::default())
            .expect("capturedBase")
            .result(),
        9,
    );

    let loaded = consumer.import_parse_cache(&parsed).expect("load cache");
    let duplicate = consumer
        .add_functions_defined_in_loaded_cache(&loaded, seed_context, &QLOptions::default())
        .expect("duplicate registration result");
    assert!(duplicate.get_succ().is_empty());
    assert_eq!(duplicate.get_fail().len(), 2);
}

/// Java `SerializableParseCacheTest#tracePointsAreOptionalAndRoundTripWhenExported`。
#[test]
fn java_trace_points_are_optional_and_round_trip_when_exported() {
    let trace_producer =
        Express4Runner::with_init_options(InitOptions::builder().trace_expression(true).build());
    let traced = trace_producer
        .export_parse_cache("a && (!myTest(11) || false)")
        .expect("export traced cache");
    assert!(traced
        .trace_points
        .as_ref()
        .is_some_and(|points| !points.is_empty()));
    let parsed_traced = json_round_trip(&traced);
    assert!(parsed_traced
        .trace_points
        .as_ref()
        .is_some_and(|points| !points.is_empty()));

    let trace_consumer =
        Express4Runner::with_init_options(InitOptions::builder().trace_expression(true).build());
    assert!(trace_consumer.add_function(
        "myTest",
        |_context: &mut dyn QContext, parameters: &Parameters| {
            Ok(DataValue::Bool(matches!(
                parameters.get_value(0),
                DataValue::Int(value) if value > 10
            )))
        }
    ));
    let trace_options = QLOptions::builder().trace_expression(true).build();
    let traced_result = trace_consumer
        .execute_with_cache(
            &parsed_traced,
            context(&[("a", DataValue::Bool(true))]),
            &trace_options,
        )
        .expect("execute traced cache");
    assert_eq!(traced_result.result(), &DataValue::Bool(false));
    assert_eq!(traced_result.expression_traces().len(), 1);
    assert_eq!(
        traced_result.expression_traces()[0].to_pretty_string(0),
        concat!(
            "OPERATOR && false\n",
            "  | VARIABLE a true\n",
            "  | OPERATOR || false\n",
            "      | OPERATOR ! false\n",
            "          | FUNCTION myTest true\n",
            "              | VALUE 11 11\n",
            "      | VALUE false false\n",
        )
    );

    let plain = json_round_trip(
        &Express4Runner::new()
            .export_parse_cache("a && true")
            .expect("export plain cache"),
    );
    assert!(plain.trace_points.is_none());
    let plain_result = trace_consumer
        .execute_with_cache(
            &plain,
            context(&[("a", DataValue::Bool(true))]),
            &trace_options,
        )
        .expect("execute plain cache");
    assert_eq!(plain_result.result(), &DataValue::Bool(true));
    assert!(plain_result.expression_traces().is_empty());
}

/// Java `SerializableParseCacheTest#invalidModelsFailWithClearErrorCodes`。
#[test]
fn java_invalid_models_fail_with_clear_error_codes() {
    let runner = Express4Runner::new();

    let mut unsupported_version = runner.export_parse_cache("1 + 2").expect("export");
    unsupported_version.model_version = 2;
    assert_parse_cache_error(
        &runner,
        &unsupported_version,
        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION,
    );

    let mut unknown_opcode = runner.export_parse_cache("1 + 2").expect("export");
    find_instruction_mut(&mut unknown_opcode, "BINARY_OP").opcode = Some("UNKNOWN".to_string());
    assert_parse_cache_error(
        &runner,
        &unknown_opcode,
        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION,
    );

    let mut missing_operand = runner.export_parse_cache("a + 2").expect("export");
    find_instruction_mut(&mut missing_operand, "LOAD")
        .operands
        .as_mut()
        .expect("LOAD operands")
        .remove("name");
    assert_parse_cache_error(
        &runner,
        &missing_operand,
        error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
    );

    let mut invalid_constant = runner.export_parse_cache("1 + 2").expect("export");
    find_instruction_mut(&mut invalid_constant, "CONST")
        .operands
        .as_mut()
        .and_then(|operands| operands.get_mut("constant"))
        .and_then(serde_json::Value::as_object_mut)
        .expect("serialized constant")
        .insert(
            "type".to_string(),
            serde_json::Value::String("OBJECT".to_string()),
        );
    assert_parse_cache_error(
        &runner,
        &invalid_constant,
        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
    );

    let mut class_not_found = runner
        .export_parse_cache("new ArrayList()")
        .expect("export");
    find_instruction_mut(&mut class_not_found, "NEW_INSTANCE")
        .operands
        .as_mut()
        .expect("NEW_INSTANCE operands")
        .insert(
            "className".to_string(),
            serde_json::Value::String("no.such.Type".to_string()),
        );
    assert_parse_cache_error(
        &runner,
        &class_not_found,
        error_codes::SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND,
    );

    let mut operator_not_found = runner.export_parse_cache("1 + 2").expect("export");
    find_instruction_mut(&mut operator_not_found, "BINARY_OP")
        .operands
        .as_mut()
        .expect("BINARY_OP operands")
        .insert(
            "operator".to_string(),
            serde_json::Value::String("missing".to_string()),
        );
    assert_parse_cache_error(
        &runner,
        &operator_not_found,
        error_codes::SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND,
    );
}

struct ConstCallCompileTimeFunction;

impl CompileTimeFunction for ConstCallCompileTimeFunction {
    fn create_function_instruction(
        &self,
        function_name: &str,
        _arguments: &[&Node],
        _operator_factory: &dyn OperatorFactory,
        code_generator: &mut dyn CodeGenerator,
    ) {
        code_generator.add_instruction(Box::new(CallConstInstruction::new(
            code_generator.error_reporter(),
            Rc::new(QLambda::Empty(QLambdaEmpty::INSTANCE)),
            0,
            function_name,
        )));
    }
}

/// Java `SerializableParseCacheTest#callConstInstructionIsRejectedOnExport`。
#[test]
fn java_call_const_instruction_is_rejected_on_export() {
    let runner = Express4Runner::new();
    assert!(runner.add_compile_time_function("CONST_CALL", Rc::new(ConstCallCompileTimeFunction)));
    let error = runner
        .export_parse_cache("CONST_CALL()")
        .expect_err("CallConstInstruction must not be serializable");
    assert_eq!(
        error.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION
    );
}
