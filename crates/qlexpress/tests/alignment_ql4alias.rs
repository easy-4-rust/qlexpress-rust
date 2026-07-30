//! Stage 7: 对齐 Java `test/annotation/QL4AliasTest` (3 个 @Test)。
//!
//! QL4Alias 是 Java 注解,支持 Chinese / unicode 字段名/方法名别名。
//! Rust 端用 #[qlexpress(alias(...))] derive 属性 + register 时设置
//! NativeType.name 模拟。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::member::QLExpressNativeType;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::{Express4Runner, QLExpressType};

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.Child7")]
pub struct Child7 {
    #[qlexpress(alias("测试字段"))]
    pub value: i64,
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn runner() -> Express4Runner {
    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Child7");
    Express4Runner::with_init_options(
        InitOptions::builder()
            .class_supplier(Rc::new(supplier))
            .add_default_import(vec![QLImport::import_cls("com.example.Child7")])
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    )
}

#[test]
fn alias_field_access_chinese() {
    // 验证 Rust 端 #[qlexpress(alias)] 能产生 alias 列表 (Chinese name)
    let nt = Child7::build_native_type();
    let aliases = nt.field_aliases.get("value");
    assert!(aliases.is_some());
    assert!(aliases.unwrap().contains(&"测试字段".to_string()));
}

#[test]
fn alias_field_access_via_runner() {
    let mut runner = runner();
    runner.register_qlexpress_type::<Child7>();
    let child: DataValue = Child7 { value: 8 }.into_data_value();
    let mut ctx = HashMap::new();
    ctx.insert("c".to_string(), child);
    // 通过 derive 生成的实例字段 alias 访问。
    let r = runner
        .execute("c.测试字段", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(8));
}

#[test]
fn default_field_name_works() {
    // 默认 field name "value" 也应能访问
    let mut runner = runner();
    runner.register_qlexpress_type::<Child7>();
    let child: DataValue = Child7 { value: 42 }.into_data_value();
    let mut ctx = HashMap::new();
    ctx.insert("c".to_string(), child);
    let r = runner
        .execute("c.value", ctx, &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Long(42));
}

fn register_java_alias_contract(runner: &mut Express4Runner) {
    let mut native_type: NativeType = Child7::build_native_type();
    native_type
        .static_fields
        .insert("staticValue".to_string(), DataValue::Int(8));
    native_type
        .field_aliases
        .insert("staticValue".to_string(), vec!["测试静态字段".to_string()]);
    native_type.static_methods.insert(
        "staticMethod".to_string(),
        Rc::new(|_bean, _args| Ok(DataValue::Int(11))),
    );
    native_type.methods.insert(
        "memberMethod".to_string(),
        Rc::new(|_bean, _args| Ok(DataValue::Int(10))),
    );
    native_type
        .method_aliases
        .insert("staticMethod".to_string(), vec!["测试静态方法".to_string()]);
    native_type
        .method_aliases
        .insert("memberMethod".to_string(), vec!["测试方法".to_string()]);
    runner.register_native_type(native_type);
}

/// 逐项对应 Java `QL4AliasTest#classFieldTest`。
#[test]
fn java_alias_static_field() {
    let mut runner = runner();
    register_java_alias_contract(&mut runner);
    assert_eq!(
        runner
            .execute("Child7.测试静态字段", HashMap::new(), &opts())
            .expect("static alias field")
            .into_result(),
        DataValue::Int(8)
    );
}

/// 逐项对应 Java `QL4AliasTest#staticMethodTest`。
#[test]
fn java_alias_static_method() {
    let mut runner = runner();
    register_java_alias_contract(&mut runner);
    assert_eq!(
        runner
            .execute("Child7.测试静态方法()", HashMap::new(), &opts())
            .expect("static alias method")
            .into_result(),
        DataValue::Int(11)
    );
}

/// 逐项对应 Java `QL4AliasTest#memberMethodTest`。
#[test]
fn java_alias_member_method() {
    let mut runner = runner();
    register_java_alias_contract(&mut runner);
    let child: DataValue = Child7 { value: 8 }.into_data_value();
    let mut context = HashMap::new();
    context.insert("child".to_string(), child);
    assert_eq!(
        runner
            .execute("child.测试方法()", context, &opts())
            .expect("member alias method")
            .into_result(),
        DataValue::Int(10)
    );
}
