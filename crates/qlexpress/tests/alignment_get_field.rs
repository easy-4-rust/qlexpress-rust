//! 逐项对齐 Java `runtime/instruction/GetFieldInstructionTest` 的 12 个测试。
//!
//! Java 依赖反射与 `allowPrivateAccess`；Rust 使用显式 `NativeType` 注册表
//! 表达相同可见性边界：未注册成员不可见，显式注册成员允许访问。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::exception::error_codes;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::member::QLExpressNativeType;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::{Express4Runner, QLExpressType};

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Parent")]
struct Parent {
    age: i32,
    name: String,
    #[qlexpress(readonly)]
    birth: String,
}

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Child")]
struct Child {
    age: i32,
    #[qlexpress(readonly)]
    birth: String,
}

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.TestEnumValue")]
struct TestEnumValue {
    #[qlexpress(readonly)]
    value: i32,
}

fn options() -> QLOptions {
    QLOptions::builder().build()
}

fn runner_with_types(types: Vec<NativeType>) -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    for name in [
        "com.example.Parent",
        "com.example.Child",
        "com.example.TestEnum",
        "com.example.TestEnumValue",
    ] {
        supplier.register(name);
    }
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![
                QLImport::import_cls("com.example.Parent"),
                QLImport::import_cls("com.example.Child"),
                QLImport::import_cls("com.example.TestEnum"),
            ])
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    for native_type in types {
        runner.register_native_type(native_type);
    }
    runner
}

fn parent_type(expose_private_static: bool, expose_private_name: bool) -> NativeType {
    let mut native_type = Parent::build_native_type();
    native_type.static_fields.insert(
        "staticGet".to_string(),
        DataValue::Str("staticGet1".to_string()),
    );
    native_type.static_field_cells.insert(
        "staticSet".to_string(),
        Rc::new(RefCell::new(DataValue::Str("staticSet".to_string()))),
    );
    // Java 允许通过实例读取静态 getter。
    native_type.fields.insert(
        "staticGet".to_string(),
        Rc::new(|_| Some(DataValue::Str("staticGet1".to_string()))),
    );
    if expose_private_static {
        native_type.static_fields.insert(
            "staticSetPrivate".to_string(),
            DataValue::Str("staticSetPrivate".to_string()),
        );
    }
    if !expose_private_name {
        native_type.fields.remove("name");
        native_type.field_setters.remove("name");
    }
    native_type
}

fn parent_value() -> DataValue {
    Parent {
        age: 0,
        name: "name".to_string(),
        birth: "2022-01-01".to_string(),
    }
    .into_data_value()
}

fn child_value() -> DataValue {
    Child {
        age: 11,
        birth: "2022-01-01".to_string(),
    }
    .into_data_value()
}

fn context(name: &str, value: DataValue) -> HashMap<String, DataValue> {
    HashMap::from([(name.to_string(), value)])
}

fn assert_field_not_found(result: Result<qlexpress::QLResult, qlexpress::exception::QLException>) {
    let error = result.expect_err("field lookup must fail");
    assert_eq!(error.error_code(), error_codes::FIELD_NOT_FOUND);
}

/// Java `GetFieldInstructionTest#case1`：通过类对象读取只读静态字段。
#[test]
fn java_case1_static_getter_field() {
    let runner = runner_with_types(vec![parent_type(false, false)]);
    let result = runner
        .execute("Parent.staticGet", HashMap::new(), &options())
        .expect("static getter must resolve")
        .into_result();
    assert_eq!(result, DataValue::Str("staticGet1".to_string()));
}

/// Java `GetFieldInstructionTest#case2`：静态字段必须返回可写左值。
#[test]
fn java_case2_public_static_field_is_writable_left_value() {
    let runner = runner_with_types(vec![parent_type(false, false)]);
    let result = runner
        .execute(
            "Parent.staticSet = 'staticSet1'; Parent.staticSet",
            HashMap::new(),
            &options(),
        )
        .expect("static assignment must succeed")
        .into_result();
    assert_eq!(result, DataValue::Str("staticSet1".to_string()));
}

/// Java `GetFieldInstructionTest#case3`：不可见私有静态字段视为不存在。
#[test]
fn java_case3_private_static_field_is_not_found() {
    let runner = runner_with_types(vec![parent_type(false, false)]);
    assert_field_not_found(runner.execute("Parent.staticSetPrivate", HashMap::new(), &options()));
}

/// Java `GetFieldInstructionTest#case4`：显式暴露等价于允许私有访问。
#[test]
fn java_case4_private_static_field_with_explicit_access() {
    let runner = runner_with_types(vec![parent_type(true, false)]);
    let result = runner
        .execute("Parent.staticSetPrivate", HashMap::new(), &options())
        .expect("explicitly exposed static field must resolve")
        .into_result();
    assert_eq!(result, DataValue::Str("staticSetPrivate".to_string()));
}

/// Java `GetFieldInstructionTest#case5`：实例可读取静态 getter 属性。
#[test]
fn java_case5_static_getter_via_instance() {
    let runner = runner_with_types(vec![parent_type(false, false)]);
    let result = runner
        .execute(
            "parent.staticGet",
            context("parent", parent_value()),
            &options(),
        )
        .expect("instance static getter must resolve")
        .into_result();
    assert_eq!(result, DataValue::Str("staticGet1".to_string()));
}

/// Java `GetFieldInstructionTest#case6`：getter/setter 属性必须可写。
#[test]
fn java_case6_instance_property_is_writable_left_value() {
    let runner = runner_with_types(vec![parent_type(false, false)]);
    let result = runner
        .execute(
            "parent.age = 35; parent.age",
            context("parent", parent_value()),
            &options(),
        )
        .expect("instance assignment must succeed")
        .into_result();
    assert_eq!(result, DataValue::Int(35));
}

/// Java `GetFieldInstructionTest#case7`：允许私有访问后字段可写。
#[test]
fn java_case7_private_instance_field_with_explicit_access() {
    let runner = runner_with_types(vec![parent_type(false, true)]);
    let result = runner
        .execute(
            "parent.name = 'name1'; parent.name",
            context("parent", parent_value()),
            &options(),
        )
        .expect("explicitly exposed instance field must be writable")
        .into_result();
    assert_eq!(result, DataValue::Str("name1".to_string()));
}

/// Java `GetFieldInstructionTest#case8`：未暴露私有实例字段视为不存在。
#[test]
fn java_case8_private_instance_field_is_not_found() {
    let runner = runner_with_types(vec![parent_type(false, false)]);
    assert_field_not_found(runner.execute(
        "parent.name",
        context("parent", parent_value()),
        &options(),
    ));
}

/// Java `GetFieldInstructionTest#case9`：子类 getter 覆盖父类 getter。
#[test]
fn java_case9_child_property_overrides_parent() {
    let runner = runner_with_types(vec![Child::build_native_type()]);
    let result = runner
        .execute("child.age", context("child", child_value()), &options())
        .expect("child getter must resolve")
        .into_result();
    assert_eq!(result, DataValue::Int(11));
}

/// Java `GetFieldInstructionTest#case10`：可见父级 getter 优先于私有字段。
#[test]
fn java_case10_inherited_public_getter_is_visible() {
    let runner = runner_with_types(vec![Child::build_native_type()]);
    let result = runner
        .execute("child.birth", context("child", child_value()), &options())
        .expect("inherited getter contract must resolve")
        .into_result();
    assert_eq!(result, DataValue::Str("2022-01-01".to_string()));
}

/// Java `GetFieldInstructionTest#case11`：普通方法不能冒充属性。
#[test]
fn java_case11_method_name_is_not_a_field() {
    let runner = runner_with_types(vec![Child::build_native_type()]);
    assert_field_not_found(runner.execute(
        "child.method1",
        context("child", child_value()),
        &options(),
    ));
}

/// Java `GetFieldInstructionTest#case12`：枚举常量后继续读取实例属性。
#[test]
fn java_case12_enum_static_member_then_instance_field() {
    let enum_value = TestEnumValue { value: -1 }.into_data_value();
    let mut enum_type = NativeType::named("com.example.TestEnum");
    enum_type
        .static_fields
        .insert("SKT".to_string(), enum_value);
    let runner = runner_with_types(vec![enum_type, TestEnumValue::build_native_type()]);
    let result = runner
        .execute("TestEnum.SKT.value", HashMap::new(), &options())
        .expect("enum static member and value getter must resolve")
        .into_result();
    assert_eq!(result, DataValue::Int(-1));
}

/// Rust 增量：派生对象仍保留直接字段访问契约。
#[test]
fn rust_native_object_direct_field_access() {
    let parent = parent_value();
    let object: &dyn NativeObject = &*parent.as_object_ref().unwrap().borrow();
    assert_eq!(object.get_field("age"), Some(DataValue::Int(0)));
    assert_eq!(object.get_field("missing"), None);
}
