//! 具名声明类型与 Java `Class<?>` 约束的端到端语义对齐。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::exception::error_codes;
use qlexpress::exception::QLException;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::context::{ExpressContext, MapExpressContext};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

struct TypedFixture {
    type_name: &'static str,
}

impl NativeObject for TypedFixture {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(
        &mut self,
        name: &str,
        _args: &[DataValue],
    ) -> Result<DataValue, QLException> {
        Err(QLException::for_test(
            qlexpress::exception::ql_exception::QLExceptionKind::Runtime,
            format!("method not found: {name}"),
            error_codes::METHOD_NOT_FOUND,
        ))
    }

    fn native_type_name(&self) -> &str {
        self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn child_value() -> DataValue {
    DataValue::Object(Rc::new(RefCell::new(TypedFixture {
        type_name: "test.Child",
    })))
}

fn runner() -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("test.Parent");
    supplier.register("test.Child");
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![QLImport::import_cls("test.Parent")])
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    runner.register_native_type(NativeType::named("test.Parent"));
    let mut child = NativeType::named("test.Child");
    child.supertypes.push("test.Parent".to_string());
    runner.register_native_type(child);
    runner
}

fn express_context(name: &str, value: DataValue) -> Rc<dyn ExpressContext> {
    Rc::new(MapExpressContext::new(Rc::new(RefCell::new(
        IndexMap::from_entries(vec![(DataValue::Str(name.to_string()), value)]),
    ))))
}

/// SOURCE_PARITY: `QLambdaInner#inheritScope` 对具名参数执行
/// `Parent.class.isInstance(child)`，而不是降级为 Object。
#[test]
fn named_function_parameter_accepts_registered_subclass_and_rejects_unrelated_value() {
    let runner = runner();
    let script = "function accept(Parent p) { return p != null; } accept(value);";
    let mut accepted = HashMap::new();
    accepted.insert("value".to_string(), child_value());
    let result = runner
        .execute(script, accepted, &QLOptions::default())
        .expect("registered child is assignable to parent")
        .into_result();
    assert_eq!(result, DataValue::Bool(true));

    let mut rejected = HashMap::new();
    rejected.insert(
        "value".to_string(),
        DataValue::Str("unrelated".to_string()),
    );
    let error = runner
        .execute(script, rejected, &QLOptions::default())
        .unwrap_err();
    assert_eq!(error.error_code(), error_codes::INVALID_ARGUMENT);
}

/// SOURCE_PARITY: `DefineLocalInstruction` 与后续 `LeftValue#set` 都保留
/// 完整具名声明类型。
#[test]
fn named_local_variable_enforces_type_on_initialization_and_reassignment() {
    let runner = runner();
    let mut context = HashMap::new();
    context.insert("value".to_string(), child_value());
    let result = runner
        .execute(
            "Parent p = value; p = value; p != null;",
            context,
            &QLOptions::default(),
        )
        .expect("registered child remains assignable on reassignment")
        .into_result();
    assert_eq!(result, DataValue::Bool(true));

    let mut rejected = HashMap::new();
    rejected.insert(
        "value".to_string(),
        DataValue::Str("unrelated".to_string()),
    );
    let error = runner
        .execute(
            "Parent p = value;",
            rejected,
            &QLOptions::default(),
        )
        .unwrap_err();
    assert_eq!(
        error.error_code(),
        error_codes::INCOMPATIBLE_ASSIGNMENT_TYPE
    );
}

/// SOURCE_PARITY: `CastInstruction` 的引用类型转换同样使用
/// `Class#isInstance`，允许注册子类向父类转换。
#[test]
fn named_cast_accepts_registered_subclass_and_rejects_unrelated_value() {
    let runner = runner();
    let mut accepted = HashMap::new();
    accepted.insert("value".to_string(), child_value());
    let result = runner
        .execute(
            "(Parent) value != null;",
            accepted,
            &QLOptions::default(),
        )
        .expect("registered child cast to parent")
        .into_result();
    assert_eq!(result, DataValue::Bool(true));

    let mut rejected = HashMap::new();
    rejected.insert(
        "value".to_string(),
        DataValue::Str("unrelated".to_string()),
    );
    let error = runner
        .execute("(Parent) value;", rejected, &QLOptions::default())
        .unwrap_err();
    assert_eq!(error.error_code(), error_codes::INCOMPATIBLE_TYPE_CAST);
}

/// SOURCE_PARITY: `SerializableParseCacheExporter/Importer` 必须往返保存
/// `Param.clazz` 的具名类，而不是导出后还原成 Object。
#[test]
fn serializable_parse_cache_preserves_named_parameter_class() {
    let runner = runner();
    let script = "function accept(Parent p) { return p != null; } accept(value);";
    let cache = runner
        .export_parse_cache(script)
        .expect("named parameter cache export");
    let json = serde_json::to_string(&cache).expect("serialize cache");
    assert!(json.contains("test.Parent"));
    let restored = serde_json::from_str(&json).expect("deserialize cache");

    let result = runner
        .execute_with_cache(
            &restored,
            express_context("value", child_value()),
            &QLOptions::default(),
        )
        .expect("named parameter cache import and execute")
        .into_result();
    assert_eq!(result, DataValue::Bool(true));
}
