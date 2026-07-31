//! Stage 5b:api/parsecache 编译缓存「导出 → JSON → 导入」往返测试。
//! 对齐 Java `SerializableParseCacheExporter` / `SerializableParseCacheImporter`
//! 的语义:常量、指令(含嵌套 Lambda)、trace 点、错误分支。

use std::rc::Rc;

use qlexpress::aparser::compile_cache::QCompileCache;
use qlexpress::aparser::operator_factory::OperatorFactory;
use qlexpress::api::parsecache::{
    LoadedCompileCache, SerializableParseCache, SerializableParseCacheExporter,
    SerializableParseCacheImporter, MODEL_VERSION,
};
use qlexpress::class_supplier::DefaultClassSupplier;
use qlexpress::exception::default_err_reporter::DefaultErrReporter;
use qlexpress::exception::error_codes;
use qlexpress::exception::error_reporter::ErrorReporter;
use qlexpress::runtime::data::convert::obj_type_convertor::TargetType;
use qlexpress::runtime::instruction::{
    ConstInstruction, DefineFunctionInstruction, Instruction, LoadInstruction,
    LoadLambdaInstruction, OperatorInstruction, ReturnInstruction, ReturnResultType,
    WhileInstruction,
};
use qlexpress::runtime::member::{as_meta_class, ClassRef, MetaClass};
use qlexpress::runtime::operator::operator_manager::OperatorManager;
use qlexpress::runtime::qlambda_definition::QLambdaDefinition;
use qlexpress::runtime::qlambda_definition_inner::{Param, QLambdaDefinitionInner};
use qlexpress::runtime::trace::{TracePointTree, TraceType};
use qlexpress::runtime::value::DataValue;

const SCRIPT: &str = "a + 2";

fn reporter() -> Rc<dyn ErrorReporter> {
    Rc::new(DefaultErrReporter::new(SCRIPT, 0, 1, 1, ""))
}

/// 构造主 Lambda:`return a + 2`(CONST/CONST/BINARY_OP/RETURN 四条指令)。
fn build_main_definition(manager: &OperatorManager) -> Rc<dyn QLambdaDefinition> {
    let plus = manager.get_binary_operator("+").expect("内建 + 操作符");
    let instructions: Vec<Instruction> = vec![
        Box::new(LoadInstruction::new(reporter(), "a", None)),
        Box::new(ConstInstruction::new(reporter(), DataValue::Int(2), None)),
        Box::new(OperatorInstruction::new(reporter(), plus, None)),
        Box::new(ReturnInstruction::new(
            reporter(),
            ReturnResultType::Return,
            None,
        )),
    ];
    Rc::new(QLambdaDefinitionInner::new(
        "main",
        instructions,
        vec![Param::new("a", Some(ClassRef::Primitive(TargetType::Int)))],
        2,
    ))
}

fn build_compile_cache(manager: &OperatorManager) -> LoadedCompileCache {
    let trace = TracePointTree::new(TraceType::Operator, "+", vec![], 1, 5, 4);
    QCompileCache::new(build_main_definition(manager), vec![trace])
}

#[test]
fn export_json_import_round_trip() {
    let manager = OperatorManager::new();
    let cache = build_compile_cache(&manager);

    // 导出(含 trace 点)
    let exporter = SerializableParseCacheExporter::new(SCRIPT, &manager, true);
    let exported = exporter.export(&cache).expect("导出成功");
    assert_eq!(exported.model_version, MODEL_VERSION);
    assert_eq!(exported.script.as_deref(), Some(SCRIPT));
    assert_eq!(exported.script_hash.as_ref().unwrap().len(), 64); // SHA-256 hex
    let main = exported.main.as_ref().unwrap();
    assert_eq!(main.name.as_deref(), Some("main"));
    assert_eq!(main.max_stack_size, 2);
    assert_eq!(main.params.as_ref().unwrap()[0].name.as_deref(), Some("a"));
    assert_eq!(
        main.params.as_ref().unwrap()[0].class_name.as_deref(),
        // Java `int.class.getName()` returns the primitive name, not the
        // boxed `Integer` class name.
        Some("int")
    );
    let opcodes: Vec<&str> = main
        .instructions
        .as_ref()
        .unwrap()
        .iter()
        .map(|inst| inst.opcode.as_deref().unwrap())
        .collect();
    assert_eq!(opcodes, vec!["LOAD", "CONST", "BINARY_OP", "RETURN"]);
    assert_eq!(exported.trace_points.as_ref().unwrap().len(), 1);

    // JSON 往返(serde,Jackson 的 Rust 对应物)
    let json = serde_json::to_string_pretty(&exported).expect("JSON 序列化");
    let parsed: SerializableParseCache = serde_json::from_str(&json).expect("JSON 反序列化");
    assert_eq!(parsed, exported, "JSON 往返后结构完全一致");

    // 导入还原
    let supplier = DefaultClassSupplier::instance();
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let loaded = importer.load(&parsed, 42).expect("导入成功");
    assert_eq!(loaded.get_model_version(), MODEL_VERSION);
    assert_eq!(loaded.get_script(), Some(SCRIPT));
    assert!(loaded.has_trace_points());
    assert!(loaded.is_bound_to(42));
    assert!(!loaded.is_bound_to(7));

    // 还原后的编译产物:主 Lambda 指令逐条对应
    let restored_main = loaded.get_compile_cache().q_lambda_definition();
    assert_eq!(restored_main.name(), "main");
    let restored_inner = restored_main
        .as_any()
        .and_then(|any| any.downcast_ref::<QLambdaDefinitionInner>())
        .expect("还原为 QLambdaDefinitionInner");
    assert_eq!(restored_inner.instructions().len(), 4);
    assert_eq!(restored_inner.max_stack_size(), 2);
    assert_eq!(restored_inner.params_type()[0].name(), "a");
    assert_eq!(
        restored_inner.params_type()[0].clazz(),
        Some(&ClassRef::Primitive(TargetType::Int))
    );
    let const_inst = restored_inner.instructions()[1]
        .as_any()
        .and_then(|any| any.downcast_ref::<ConstInstruction>())
        .expect("第二条为 CONST");
    assert_eq!(const_inst.const_obj(), &DataValue::Int(2));

    // trace 点还原
    let traces = loaded.get_compile_cache().expression_trace_points();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].token(), "+");
    assert_eq!(traces[0].trace_type(), TraceType::Operator);

    // 二次导出与首次导出一致(幂等)
    let re_exporter = SerializableParseCacheExporter::new(SCRIPT, &manager, true);
    let re_exported = re_exporter
        .export(loaded.get_compile_cache())
        .expect("二次导出");
    assert_eq!(re_exported.main, exported.main);
}

/// 常量全类型往返:NULL/BOOLEAN/STRING/CHAR/INT/LONG/BIG_INTEGER/FLOAT/
/// DOUBLE/BIG_DECIMAL/META_CLASS(对齐 Java `exportConstant` 全分支)。
#[test]
fn constant_round_trip_all_types() {
    let manager = OperatorManager::new();
    let supplier = {
        let mut supplier = DefaultClassSupplier::instance();
        supplier.register("com.example.Widget");
        supplier
    };
    let cases: Vec<(DataValue, &str)> = vec![
        (DataValue::Null, "NULL"),
        (DataValue::Bool(true), "BOOLEAN"),
        (DataValue::Str("hello".into()), "STRING"),
        (DataValue::Char('x' as u16), "CHAR"),
        (DataValue::Int(-7), "INT"),
        (DataValue::Long(9_000_000_000), "LONG"),
        (
            DataValue::big_int(123456789012345678901234567890i128),
            "BIG_INTEGER",
        ),
        (DataValue::Float(1.5), "FLOAT"),
        (DataValue::Double(-2.25), "DOUBLE"),
        (
            DataValue::BigDec("3.141592653589793238462643".to_string()),
            "BIG_DECIMAL",
        ),
        (
            MetaClass::new(ClassRef::Named("com.example.Widget".to_string())).into_data_value(),
            "META_CLASS",
        ),
    ];

    for (value, expected_type) in cases {
        let instructions: Vec<Instruction> = vec![
            Box::new(ConstInstruction::new(reporter(), value.clone(), None)),
            Box::new(ReturnInstruction::new(
                reporter(),
                ReturnResultType::Return,
                None,
            )),
        ];
        let cache: LoadedCompileCache = QCompileCache::new(
            Rc::new(QLambdaDefinitionInner::new("main", instructions, vec![], 1)),
            vec![],
        );
        let exporter = SerializableParseCacheExporter::new(SCRIPT, &manager, false);
        let exported = exporter.export(&cache).expect("导出常量");
        let main = exported.main.as_ref().unwrap();
        let const_operands = main.instructions.as_ref().unwrap()[0]
            .operands
            .as_ref()
            .unwrap();
        let constant = const_operands.get("constant").unwrap();
        assert_eq!(
            constant.get("type").and_then(|t| t.as_str()),
            Some(expected_type),
            "类型标签: {expected_type}"
        );

        // JSON 往返 + 导入
        let json = serde_json::to_string(&exported).unwrap();
        let parsed: SerializableParseCache = serde_json::from_str(&json).unwrap();
        let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
        let loaded = importer.load(&parsed, 1).expect("导入常量");
        let restored = loaded.get_compile_cache().q_lambda_definition();
        let inner = restored
            .as_any()
            .and_then(|any| any.downcast_ref::<QLambdaDefinitionInner>())
            .unwrap();
        let const_inst = inner.instructions()[0]
            .as_any()
            .and_then(|any| any.downcast_ref::<ConstInstruction>())
            .unwrap();
        match (&value, expected_type) {
            (DataValue::Float(v), _) => assert_eq!(const_inst.const_obj(), &DataValue::Float(*v)),
            (_, "META_CLASS") => {
                let class_ref = as_meta_class(const_inst.const_obj()).expect("MetaClass 还原");
                assert_eq!(class_ref.java_name(), "com.example.Widget");
            }
            _ => assert_eq!(const_inst.const_obj(), &value, "常量往返: {expected_type}"),
        }
    }
}

/// 嵌套 Lambda 往返:DEFINE_FUNCTION(命名 lambda)+ WHILE(condition/body)。
#[test]
fn nested_lambda_definitions_round_trip() {
    let manager = OperatorManager::new();
    let cond: Rc<dyn QLambdaDefinition> = Rc::new(QLambdaDefinitionInner::new(
        "while$condition",
        vec![Box::new(ConstInstruction::new(
            reporter(),
            DataValue::Bool(true),
            None,
        )) as Instruction],
        vec![],
        1,
    ));
    let body: Rc<dyn QLambdaDefinition> =
        Rc::new(QLambdaDefinitionInner::new("while$body", vec![], vec![], 0));
    let func_lambda: Rc<dyn QLambdaDefinition> = Rc::new(QLambdaDefinitionInner::new(
        "f",
        vec![Box::new(LoadInstruction::new(reporter(), "x", None)) as Instruction],
        vec![Param::new("x", None)],
        1,
    ));
    let instructions: Vec<Instruction> = vec![
        Box::new(DefineFunctionInstruction::new(reporter(), "f", func_lambda)),
        Box::new(WhileInstruction::new(reporter(), cond, body, 1)),
        Box::new(ReturnInstruction::new(
            reporter(),
            ReturnResultType::Return,
            None,
        )),
    ];
    let cache: LoadedCompileCache = QCompileCache::new(
        Rc::new(QLambdaDefinitionInner::new("main", instructions, vec![], 1)),
        vec![],
    );

    let exporter = SerializableParseCacheExporter::new(SCRIPT, &manager, false);
    let exported = exporter.export(&cache).expect("导出嵌套 lambda");
    let json = serde_json::to_string(&exported).unwrap();
    let parsed: SerializableParseCache = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, exported);

    let supplier = DefaultClassSupplier::instance();
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let loaded = importer.load(&parsed, 2).expect("导入嵌套 lambda");
    let restored = loaded.get_compile_cache().q_lambda_definition();
    let inner = restored
        .as_any()
        .and_then(|any| any.downcast_ref::<QLambdaDefinitionInner>())
        .unwrap();
    // DEFINE_FUNCTION 的名字与嵌套 lambda 还原
    let define_fn = inner.instructions()[0]
        .as_any()
        .and_then(|any| any.downcast_ref::<DefineFunctionInstruction>())
        .expect("DEFINE_FUNCTION 还原");
    assert_eq!(define_fn.name(), "f");
    assert_eq!(define_fn.lambda_definition().name(), "f");
    // WHILE 的 condition/body 还原
    let while_inst = inner.instructions()[1]
        .as_any()
        .and_then(|any| any.downcast_ref::<WhileInstruction>())
        .expect("WHILE 还原");
    assert_eq!(while_inst.condition().name(), "while$condition");
    assert_eq!(while_inst.while_scope_max_stack_size(), 1);
    // LOAD_LAMBDA 覆盖:直接在主序列中加载 lambda
    let load_lambda_cache: LoadedCompileCache = QCompileCache::new(
        Rc::new(QLambdaDefinitionInner::new(
            "main",
            vec![
                Box::new(LoadLambdaInstruction::new(
                    reporter(),
                    Rc::new(QLambdaDefinitionInner::new("g", vec![], vec![], 0)),
                )) as Instruction,
                Box::new(ReturnInstruction::new(
                    reporter(),
                    ReturnResultType::Return,
                    None,
                )),
            ],
            vec![],
            1,
        )),
        vec![],
    );
    let exported2 = exporter.export(&load_lambda_cache).unwrap();
    let parsed2: SerializableParseCache =
        serde_json::from_str(&serde_json::to_string(&exported2).unwrap()).unwrap();
    let mut importer2 = SerializableParseCacheImporter::new(&manager, &supplier);
    let loaded2 = importer2.load(&parsed2, 2).expect("LOAD_LAMBDA 导入");
    assert_eq!(
        loaded2.get_compile_cache().q_lambda_definition().name(),
        "main"
    );
}

/// 错误分支:版本不符 / 缺 script / 未知 opcode / 未知操作符。
#[test]
fn import_error_branches() {
    let manager = OperatorManager::new();
    let supplier = DefaultClassSupplier::instance();
    let cache = build_compile_cache(&manager);
    let exporter = SerializableParseCacheExporter::new(SCRIPT, &manager, false);
    let exported = exporter.export(&cache).unwrap();

    // 模型版本不符 → UNSUPPORTED_VERSION
    let mut bad_version = exported.clone();
    bad_version.model_version = 999;
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let err = importer.load(&bad_version, 0).err().unwrap();
    assert_eq!(
        err.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION
    );

    // 缺 script → INVALID_MODEL
    let mut no_script = exported.clone();
    no_script.script = None;
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let err = importer.load(&no_script, 0).err().unwrap();
    assert_eq!(
        err.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL
    );

    // 未知 opcode → UNSUPPORTED_INSTRUCTION
    let mut bad_opcode = exported.clone();
    bad_opcode
        .main
        .as_mut()
        .unwrap()
        .instructions
        .as_mut()
        .unwrap()[0]
        .opcode = Some("NO_SUCH_OP".to_string());
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let err = importer.load(&bad_opcode, 0).err().unwrap();
    assert_eq!(
        err.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION
    );

    // 未知操作符 → OPERATOR_NOT_FOUND
    let mut bad_operator = exported.clone();
    let operands = bad_operator
        .main
        .as_mut()
        .unwrap()
        .instructions
        .as_mut()
        .unwrap()[2]
        .operands
        .as_mut()
        .unwrap();
    operands.insert(
        "operator".to_string(),
        serde_json::Value::from("###nope###"),
    );
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let err = importer.load(&bad_operator, 0).err().unwrap();
    assert_eq!(
        err.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND
    );

    // 未注册类型(META_CLASS)→ CLASS_NOT_FOUND
    let mut bad_class = exported.clone();
    let const_operands = bad_class
        .main
        .as_mut()
        .unwrap()
        .instructions
        .as_mut()
        .unwrap()[1]
        .operands
        .as_mut()
        .unwrap();
    let constant = const_operands.get_mut("constant").unwrap();
    *constant = serde_json::json!({"type": "META_CLASS", "value": "com.example.Missing"});
    let mut importer = SerializableParseCacheImporter::new(&manager, &supplier);
    let err = importer.load(&bad_class, 0).err().unwrap();
    assert_eq!(
        err.error_code(),
        error_codes::SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND
    );
}
